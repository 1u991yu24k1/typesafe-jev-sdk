use std::env;
use std::time::Duration;

use reqwest::Url;

use crate::error::JevError;

/// Connection settings. The endpoint is a complete `/v1/systemone` URL.
#[derive(Clone)]
pub struct Config {
    pub(crate) endpoint: Url,
    pub(crate) api_key: String,
    pub(crate) proxy: Option<String>,
    pub(crate) connect_timeout: Duration,
    pub(crate) request_timeout: Duration,
    pub(crate) allow_http_for_testing: bool,
}

impl Config {
    pub fn new(endpoint: &str, api_key: impl Into<String>) -> Result<Self, JevError> {
        let endpoint = Url::parse(endpoint).map_err(|_| JevError::InvalidUrl)?;
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(JevError::MissingApiKey);
        }
        let config = Self {
            endpoint,
            api_key,
            proxy: None,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            allow_http_for_testing: false,
        };
        config.validate_key()?;
        config.validate_endpoint()?;
        Ok(config)
    }

    pub fn from_env(connect_timeout: Duration) -> Result<Self, JevError> {
        let endpoint = env::var("TYPESAFE_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.typesafe.ai/v1/systemone".to_owned());
        let api_key = env::var("TYPESAFE_API_KEY").map_err(|_| JevError::MissingApiKey)?;
        let mut config = Self::new(&endpoint, api_key)?;
        config.connect_timeout = connect_timeout;
        config.proxy = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
            .into_iter()
            .find_map(|name| env::var(name).ok());
        Ok(config)
    }

    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxy = Some(proxy.into());
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Only for explicitly configured local mock servers.
    pub fn allow_http_for_testing(mut self) -> Self {
        self.allow_http_for_testing = true;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), JevError> {
        self.validate_key()?;
        self.validate_endpoint()?;
        if self.connect_timeout.is_zero() || self.request_timeout.is_zero() {
            return Err(JevError::Validation("timeouts must be greater than zero"));
        }
        Ok(())
    }

    fn validate_key(&self) -> Result<(), JevError> {
        if self.api_key.trim().is_empty() {
            return Err(JevError::MissingApiKey);
        }
        // Restrict to visible ASCII so bearer header construction cannot fail
        // later or embed control characters. Never include the key in errors.
        if !self.api_key.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
            return Err(JevError::InvalidApiKey);
        }
        Ok(())
    }

    fn validate_endpoint(&self) -> Result<(), JevError> {
        if !matches!(self.endpoint.scheme(), "http" | "https")
            || self.endpoint.host_str().is_none()
            || !self.endpoint.username().is_empty()
            || self.endpoint.password().is_some()
            || self.endpoint.fragment().is_some()
        {
            return Err(JevError::InvalidUrl);
        }
        Ok(())
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("endpoint", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .field("proxy", &self.proxy.as_ref().map(|_| "[REDACTED]"))
            .field("connect_timeout", &self.connect_timeout)
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_credentials_urls_and_zero_timeouts() {
        for key in ["test\nkey", "test\rkey", "test key", "テスト", "test\0key"] {
            let error = Config::new("https://example.invalid", key).unwrap_err();
            assert!(matches!(error, JevError::InvalidApiKey));
            assert!(!format!("{error:?} {error}").contains(key));
        }
        assert!(matches!(
            Config::new("https://example.invalid", "  "),
            Err(JevError::MissingApiKey)
        ));
        for url in [
            "ftp://example.invalid",
            "https://user:password@example.invalid",
            "https://example.invalid/#fragment",
        ] {
            assert!(matches!(
                Config::new(url, "test-key"),
                Err(JevError::InvalidUrl)
            ));
        }
        let config = || Config::new("https://example.invalid", "test-key").unwrap();
        for config in [
            config().connect_timeout(Duration::ZERO),
            config().request_timeout(Duration::ZERO),
        ] {
            assert!(matches!(
                crate::client::TypeSafeClient::try_new(config),
                Err(JevError::Validation(_))
            ));
        }
        assert!(matches!(
            crate::client::TypeSafeClient::try_new(
                Config::new("http://127.0.0.1", "test-key").unwrap()
            ),
            Err(JevError::InvalidUrl)
        ));
    }

    #[test]
    fn fallible_config_distinguishes_missing_key_and_invalid_url() {
        assert!(matches!(
            Config::new("https://example.invalid", ""),
            Err(JevError::MissingApiKey)
        ));
        assert!(matches!(
            Config::new("not a url", "secret"),
            Err(JevError::InvalidUrl)
        ));
    }

    #[test]
    fn client_rejects_bad_proxy_without_leaking_secrets() {
        let config = Config::new("https://example.invalid/v1/systemone", "top-secret")
            .unwrap()
            .proxy("http://[");
        assert!(!format!("{config:?}").contains("top-secret"));
        assert!(matches!(
            crate::client::TypeSafeClient::try_new(config),
            Err(JevError::InvalidProxy)
        ));
    }
}
