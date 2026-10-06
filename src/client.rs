use std::fmt;
use std::time::Duration;

use super::config::Config;
use super::error::JevError;
use super::request::JevRequest;
use super::response::JevResponse;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Proxy, Url};

pub struct TypeSafeClient {
    pub client: Client,
    pub url: Url,
    api_key: String,
}

impl TypeSafeClient {
    pub fn new(timeout: Duration) -> Self {
        Self::try_new(Config::from_env(timeout).expect("invalid TypeSafe SDK configuration"))
            .expect("failed to construct TypeSafe SDK client")
    }

    pub fn try_new(config: Config) -> Result<Self, JevError> {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let mut client_builder = reqwest::ClientBuilder::new()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .default_headers(headers)
            .https_only(!config.allow_http_for_testing);

        if config.endpoint.scheme() != "https"
            && !(config.allow_http_for_testing && config.endpoint.scheme() == "http")
        {
            return Err(JevError::InvalidUrl);
        }
        if let Some(ref proxy_url) = config.proxy {
            let p = Proxy::all(proxy_url).map_err(|_| JevError::InvalidProxy)?;
            client_builder = client_builder.proxy(p)
        }

        let client = client_builder.build().map_err(JevError::ClientBuild)?;
        Ok(Self {
            client,
            url: config.endpoint,
            api_key: config.api_key,
        })
    }

    pub async fn system_one(&self, request: &JevRequest) -> Result<JevResponse, reqwest::Error> {
        self.client
            .post(self.url.clone())
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?
            .json::<JevResponse>()
            .await
    }
}

impl Default for TypeSafeClient {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

impl fmt::Debug for TypeSafeClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeSafeClient")
            .field("client", &self.client)
            .field("url", &"[REDACTED]")
            .field("apikey", &"***********")
            .finish()
    }
}

#[cfg(test)]
mod client_tests {
    use super::*;
    use crate::answer::Answer;
    use crate::builder::JevRequestBuilder;
    use crate::question::Question;
    use std::env;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    // Use an ephemeral loopback server, never the live API or environment keys.
    async fn mock_client(status: &str, body: &str) -> (TypeSafeClient, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!(
            "http://{}/systemone",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let size = stream.read(&mut buffer).await.unwrap();
                assert_ne!(size, 0, "client closed before sending its request");
                received.extend_from_slice(&buffer[..size]);
                if let Some(header_end) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = std::str::from_utf8(&received[..header_end]).unwrap();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().unwrap())
                        })
                        .expect("JSON request has a content length");
                    if received.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            stream.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(received).unwrap()
        });
        let client = TypeSafeClient {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            url,
            api_key: "test-key".to_owned(),
        };
        (client, server)
    }

    fn request() -> JevRequest {
        JevRequestBuilder::new()
            .state("状態")
            .question(
                "ready",
                Question::Noul {
                    instructions: "Ready?".to_owned(),
                    criteria: None,
                },
            )
            .build()
            .unwrap()
    }

    const RESPONSE: &str = r#"{"model":"jev-latest","usage":{"input_tokens":2,"output_tokens":3},"answers":{"ready":{"type":"noul","noul":0.82}}}"#;

    #[tokio::test]
    async fn sends_authenticated_json_and_decodes_response() {
        let (client, server) = mock_client("200 OK", RESPONSE).await;
        let request = request();
        let response = client.system_one(&request).await.unwrap();
        let received = server.await.unwrap();
        let (headers, body) = received.split_once("\r\n\r\n").unwrap();
        let headers = headers.to_ascii_lowercase();

        assert!(headers.starts_with("post /systemone http/1.1\r\n"));
        assert!(
            headers
                .lines()
                .any(|line| line == "authorization: bearer test-key")
        );
        assert!(
            headers
                .lines()
                .any(|line| line == "content-type: application/json")
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(body).unwrap(),
            serde_json::to_value(request).unwrap()
        );
        assert_eq!(response.model, "jev-latest");
        assert_eq!(response.usage.tokens(), 5);
        assert_eq!(response.answers["ready"], Answer::Noul { noul: 0.82 });
    }

    #[tokio::test]
    async fn malformed_json_returns_decode_error() {
        let (client, server) = mock_client("200 OK", "not json").await;
        assert!(client.system_one(&request()).await.unwrap_err().is_decode());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn invalid_response_shape_returns_decode_error() {
        let (client, server) = mock_client("200 OK", "{}").await;
        assert!(client.system_one(&request()).await.unwrap_err().is_decode());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn preserves_response_decoding_on_non_success_status() {
        // This SDK currently decodes any HTTP status. A status policy change
        // should be handled separately from this compatibility refactor.
        let (client, server) = mock_client("400 Bad Request", RESPONSE).await;
        assert_eq!(
            client.system_one(&request()).await.unwrap().model,
            "jev-latest"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn https_only_transport_reports_local_http_error() {
        let (mut client, server) = mock_client("200 OK", RESPONSE).await;
        client.client = Client::builder()
            .no_proxy()
            .https_only(true)
            .build()
            .unwrap();
        let error = client.system_one(&request()).await.unwrap_err();
        server.abort();
        assert!(error.is_builder());
    }

    #[tokio::test]
    async fn stalled_response_returns_timeout_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = TypeSafeClient {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_millis(50))
                .build()
                .unwrap(),
            url: Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap(),
            api_key: "test-key".to_owned(),
        };
        // Keep the listener alive without sending a response.
        let error = client.system_one(&request()).await.unwrap_err();
        assert!(error.is_timeout());
        drop(listener);
    }

    #[test]
    fn construction_from_environment() {
        const CHILD: &str = "JEV_SDK_CONFIG_TEST";
        if let Ok(mode) = env::var(CHILD) {
            if mode.starts_with("invalid") || mode == "missing-key" {
                assert!(std::panic::catch_unwind(TypeSafeClient::default).is_err());
            } else {
                let client = TypeSafeClient::default();
                let expected = if mode == "custom-url" {
                    "https://example.invalid/custom"
                } else {
                    "https://api.typesafe.ai/v1/systemone"
                };
                assert_eq!(client.url.as_str(), expected);
                assert_eq!(client.api_key, "test-key");
            }
            return;
        }

        // Rust 2024 environment mutation is unsafe in a multithreaded process.
        // Give each configuration its own child process instead, without keys
        // or proxy settings inherited from the developer's environment.
        for mode in [
            "default",
            "custom-url",
            "missing-key",
            "invalid-url",
            "invalid-proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ] {
            let mut command = std::process::Command::new(env::current_exe().unwrap());
            command
                .env_clear()
                .env(CHILD, mode)
                .arg("--exact")
                .arg("client::client_tests::construction_from_environment");
            // Keep coverage profiles separate via cargo-llvm-cov's %p pattern.
            if let Some(profile) = env::var_os("LLVM_PROFILE_FILE") {
                command.env("LLVM_PROFILE_FILE", profile);
            }
            if mode != "missing-key" {
                command.env("TYPESAFE_API_KEY", "test-key");
            }
            match mode {
                "custom-url" => {
                    command.env("TYPESAFE_API_BASE_URL", "https://example.invalid/custom");
                }
                "invalid-url" => {
                    command.env("TYPESAFE_API_BASE_URL", "not a URL");
                }
                "invalid-proxy" => {
                    command.env("HTTPS_PROXY", "http://[");
                }
                "HTTPS_PROXY" | "https_proxy" | "HTTP_PROXY" | "http_proxy" => {
                    command.env(mode, "http://127.0.0.1:8080");
                }
                _ => {}
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{mode}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[tokio::test]
    async fn debug_output_redacts_api_key() {
        let (client, server) = mock_client("200 OK", RESPONSE).await;
        let debug = format!("{client:?}");
        server.abort();
        assert!(!debug.contains("test-key"));
        assert!(debug.contains("***********"));
    }
}
