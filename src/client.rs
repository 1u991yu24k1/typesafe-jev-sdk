use std::env;
use std::fmt;
use std::time::Duration;

use reqwest::{Url, Client, Proxy};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use super::request::JevRequest;
use super::response::JevResponse;


fn proxy() -> Result<Option<Proxy>, Box<dyn std::error::Error>> {
    let proxy_str_opt =  env::var("HTTPS_PROXY")
        .or_else(|_| env::var("https_proxy"))
        .or_else(|_| env::var("HTTP_PROXY"))
        .or_else(|_| env::var("http_proxy"))
        .ok();

    match proxy_str_opt {
        Some(s) => {
            let proxy = Proxy::all(s)?;
            Ok(Some(proxy))
        },
        None => { Ok(None) }
    }
}

pub struct TypeSafeClient {
    pub client: Client,
    pub url: Url,
    api_key: String
}

impl TypeSafeClient {
    pub fn new(timeout: Duration) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE,  HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT,        HeaderValue::from_static("application/json"));

        
        let mut client_builder = 
            reqwest::ClientBuilder::new()
            .connect_timeout(timeout)
            .default_headers(headers)
            .https_only(true);
        
        if let Some(p) = proxy().unwrap() {
            client_builder = client_builder.proxy(p)
        }
        
        let client = client_builder
            .build()
            .unwrap();
       
        let url = 
            Url::parse(
                &env::var("TYPESAFE_API_BASE_URL")
                    .unwrap_or("https://api.typesafe.ai/v1/systemone".to_string())
            ).unwrap();

        let api_key = env::var("TYPESAFE_API_KEY").unwrap();

        Self { client, url, api_key } 
    }

    pub async fn system_one(&self, request: &JevRequest) -> Result<JevResponse, reqwest::Error> {
        let resp = 
            self.client
            .post(self.url.as_str())
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?
            .json::<JevResponse>()
            .await?;

        Ok(resp)
    }
}


impl Default for TypeSafeClient {
    fn default() -> Self { Self::new(Duration::from_secs(5)) }
}

impl fmt::Debug for TypeSafeClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeSafeClient")
            .field("client", &self.client)
            .field("url", &self.url)
            .field("apikey", &"***********")
            .finish()
    }
}


#[cfg(test)]
mod client_tests {
    use super::*;
    use crate::question::Question;
    use crate::builder::JevRequestBuilder;

    #[tokio::test]
    async fn test_build_client() {
        let request = 
            JevRequestBuilder::new()
            .model("jev-latest")
            .state("プレイヤーのHPは20%。敵が近くに3体いる。\n回復アイテムを1個持っている。")
            .question(
                "next_action", 
                Question::choice(
                    "次に取る行動は?",
                    vec![
                        ("heal",    "回復アイテムを使ってHPを回復する"),
                        ("retreat", "敵から距離を取って退避する"),
                        ("attack",  "近くの敵を攻撃する")
                    ]
                )
            )
            .build()
            .unwrap();
    
        let resp = 
            TypeSafeClient::default()
            .system_one(&request)
            .await
            .unwrap();
        println!("{:#?}", resp);
    }
}