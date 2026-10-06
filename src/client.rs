use std::fmt;
use std::time::{Duration, Instant};

use super::config::Config;
use super::error::JevError;
use super::request::JevRequest;
use super::response::{JevResponse, JevResponseWithMetadata, ResponseMetadata};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Proxy, Url};

pub struct TypeSafeClient {
    client: Client,
    url: Url,
    api_key: String,
}

impl TypeSafeClient {
    /// Construct from environment settings without panicking on configuration errors.
    pub fn new(timeout: Duration) -> Result<Self, JevError> {
        Self::try_new(Config::from_env(timeout)?)
    }

    /// The complete endpoint, not a base URL. May contain confidential query values.
    pub fn endpoint(&self) -> &Url {
        &self.url
    }

    pub fn try_new(config: Config) -> Result<Self, JevError> {
        config.validate()?;
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let mut client_builder = reqwest::ClientBuilder::new()
            // Service-side idempotency and duplicate billing are not documented.
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
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

    pub async fn system_one(&self, request: &JevRequest) -> Result<JevResponse, JevError> {
        Ok(self.system_one_with_metadata(request).await?.response)
    }

    /// Send a single request and return HTTP metadata alongside the decoded response.
    ///
    /// No automatic retries are performed. HTTP status, transport and JSON errors
    /// use the same policy as `system_one`. Metadata is returned only on success.
    pub async fn system_one_with_metadata(
        &self,
        request: &JevRequest,
    ) -> Result<JevResponseWithMetadata, JevError> {
        let started = Instant::now();
        let mut response = self
            .client
            .post(self.url.clone())
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await
            .map_err(|e| JevError::Transport(e.without_url()))?;
        let status = response.status();
        if !status.is_success() {
            const MAX_ERROR_BODY_BYTES: usize = 1024;
            let mut bytes = Vec::new();
            while bytes.len() < MAX_ERROR_BODY_BYTES {
                let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|e| JevError::Transport(e.without_url()))?
                else {
                    break;
                };
                let remaining = MAX_ERROR_BODY_BYTES - bytes.len();
                bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            }
            // Remove full keys and a key prefix cut off at the capture boundary.
            // API keys are validated as ASCII header values at construction.
            let mut body = String::from_utf8_lossy(&bytes).replace(&self.api_key, "[REDACTED]");
            if bytes.len() == MAX_ERROR_BODY_BYTES {
                let prefix_len = (1..self.api_key.len())
                    .rev()
                    .find(|&len| body.ends_with(&self.api_key[..len]));
                if let Some(len) = prefix_len {
                    body.truncate(body.len() - len);
                    body.push_str("[REDACTED]");
                }
            }
            return Err(JevError::HttpStatus {
                status,
                retryable: status.as_u16() == 429 || status.is_server_error(),
                body,
            });
        }
        // JSON decoding consumes the response, so own its headers before decoding.
        let headers = response.headers().to_owned();
        let response = response
            .json::<JevResponse>()
            .await
            .map_err(|e| JevError::Decode(e.without_url()))?;
        Ok(JevResponseWithMetadata {
            response,
            metadata: ResponseMetadata::new(status, headers, started.elapsed()),
        })
    }
}

impl fmt::Debug for TypeSafeClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeSafeClient")
            .field("client", &"[REDACTED]")
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
        mock_client_with_headers(status, body, "", Duration::ZERO).await
    }

    async fn mock_client_with_headers(
        status: &str,
        body: &str,
        extra_headers: &str,
        body_delay: Duration,
    ) -> (TypeSafeClient, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!(
            "http://{}/systemone",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n",
            body.len()
        );
        let body = body.to_owned();
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
            if !body_delay.is_zero() {
                tokio::time::sleep(body_delay).await;
            }
            stream.write_all(body.as_bytes()).await.unwrap();
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
    async fn metadata_preserves_headers_and_measures_body_reception() {
        let body_delay = Duration::from_millis(20);
        let (client, server) = mock_client_with_headers(
            "201 Created",
            RESPONSE,
            "X-Test-Request-Id: test-id-not-real\r\nSet-Cookie: test-cookie-not-real\r\nSet-Cookie: second-test-cookie\r\nAuthorization: Bearer test-key\r\n",
            body_delay,
        ).await;
        let result = client.system_one_with_metadata(&request()).await.unwrap();
        server.await.unwrap();
        assert_eq!(result.metadata.status(), reqwest::StatusCode::CREATED);
        assert_eq!(
            result.metadata.request_id("x-test-request-id"),
            Some("test-id-not-real")
        );
        assert_eq!(
            result.metadata.request_id("X-Test-Request-Id"),
            Some("test-id-not-real")
        );
        assert_eq!(result.metadata.request_id("absent-id"), None);
        assert_eq!(
            result
                .metadata
                .headers()
                .get_all("set-cookie")
                .iter()
                .count(),
            2
        );
        assert!(result.metadata.elapsed() >= body_delay);
        assert_eq!(
            serde_json::to_value(&result.response).unwrap(),
            serde_json::from_str::<serde_json::Value>(RESPONSE).unwrap()
        );
        let debug = format!("{result:?}");
        for sensitive in [
            "test-key",
            "test-cookie-not-real",
            "second-test-cookie",
            "test-id-not-real",
            "jev-latest",
        ] {
            assert!(!debug.contains(sensitive));
        }
    }

    #[tokio::test]
    async fn metadata_does_not_invent_request_id() {
        let (client, server) = mock_client("200 OK", RESPONSE).await;
        let result = client.system_one_with_metadata(&request()).await.unwrap();
        assert_eq!(result.metadata.status(), reqwest::StatusCode::OK);
        assert_eq!(result.metadata.request_id("x-test-request-id"), None);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn metadata_method_preserves_error_policy() {
        let (client, server) = mock_client("429 Too Many Requests", RESPONSE).await;
        let error = client
            .system_one_with_metadata(&request())
            .await
            .unwrap_err();
        assert_eq!(error.status(), Some(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(error.retryable());
        server.await.unwrap();

        let (client, server) = mock_client("200 OK", "not json").await;
        assert!(
            client
                .system_one_with_metadata(&request())
                .await
                .unwrap_err()
                .is_decode()
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn configured_client_does_not_retry_http_errors_or_follow_post_redirects() {
        for (status, code, headers) in [
            ("429 Too Many Requests", 429, "Retry-After: 0\r\n"),
            ("503 Service Unavailable", 503, "Retry-After: 0\r\n"),
            ("307 Temporary Redirect", 307, "Location: /replayed\r\n"),
            ("308 Permanent Redirect", 308, "Location: /replayed\r\n"),
        ] {
            // The server handles one request only. Replaying it cannot return
            // this HTTP error and would instead fail to connect to the server.
            let (mock, server) =
                mock_client_with_headers(status, "", headers, Duration::ZERO).await;
            let config = Config::new(mock.url.as_str(), "test-key")
                .unwrap()
                .allow_http_for_testing();
            let client = TypeSafeClient::try_new(config).unwrap();
            let error = client.system_one(&request()).await.unwrap_err();
            assert_eq!(error.status().unwrap().as_u16(), code);
            server.await.unwrap();
        }
    }

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
    async fn non_success_is_typed_even_with_success_shaped_json() {
        for (status, code, retryable) in [
            ("400 Bad Request", 400, false),
            ("401 Unauthorized", 401, false),
            ("429 Too Many Requests", 429, true),
            ("500 Internal Server Error", 500, true),
        ] {
            let (client, server) = mock_client(status, RESPONSE).await;
            let error = client.system_one(&request()).await.unwrap_err();
            assert_eq!(error.status().unwrap().as_u16(), code);
            assert_eq!(error.retryable(), retryable);
            assert_eq!(error.error_body(), Some(RESPONSE));
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn error_body_is_bounded_and_key_is_redacted() {
        let body = format!("test-key{}", "x".repeat(3000));
        let (client, server) = mock_client("500 Internal Server Error", &body).await;
        let error = client.system_one(&request()).await.unwrap_err();
        assert!(!error.error_body().unwrap().contains("test-key"));
        assert!(!format!("{error:?}").contains("test-key"));
        assert!(error.error_body().unwrap().len() <= 1024 + 10);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn error_body_redacts_key_prefix_at_capture_boundary() {
        for prefix_len in 1.."test-key".len() {
            let body = format!("{}test-key", "x".repeat(1024 - prefix_len));
            let (client, server) = mock_client("401 Unauthorized", &body).await;
            let error = client.system_one(&request()).await.unwrap_err();
            let captured = error.error_body().unwrap();
            assert_eq!(
                captured,
                format!("{}[REDACTED]", "x".repeat(1024 - prefix_len))
            );
            assert!(!format!("{error}").contains("test-key"));
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn timeout_while_receiving_success_body_is_reported() {
        let (mock, server) =
            mock_client_with_headers("200 OK", RESPONSE, "", Duration::from_secs(2)).await;
        let config = Config::new(mock.endpoint().as_str(), "test-key")
            .unwrap()
            .allow_http_for_testing()
            .request_timeout(Duration::from_millis(50));
        let client = TypeSafeClient::try_new(config).unwrap();
        let error = client.system_one(&request()).await.unwrap_err();
        assert!(error.is_timeout());
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
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
        assert!(matches!(error, JevError::Transport(e) if e.is_builder()));
    }

    #[tokio::test]
    async fn stalled_response_returns_timeout_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = Config::new(
            &format!("http://{}", listener.local_addr().unwrap()),
            "test-key",
        )
        .unwrap()
        .allow_http_for_testing()
        .connect_timeout(Duration::from_secs(2))
        .request_timeout(Duration::from_millis(50));
        let client = TypeSafeClient::try_new(config).unwrap();
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
                assert!(TypeSafeClient::new(Duration::from_secs(5)).is_err());
            } else {
                let client = TypeSafeClient::new(Duration::from_secs(5)).unwrap();
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
