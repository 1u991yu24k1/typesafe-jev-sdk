use std::env;
use std::fmt;
use std::time::Duration;

use reqwest::{Url, Client};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use super::request::JevRequest;
use super::response::JevResponse;


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
        
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(timeout)
            .default_headers(headers)
            .https_only(true)
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