pub enum JevError {
    BuildError,
    Validation(&'static str),
    MissingApiKey,
    InvalidApiKey,
    InvalidUrl,
    InvalidProxy,
    ClientBuild(reqwest::Error),
    Transport(reqwest::Error),
    Decode(reqwest::Error),
    HttpStatus {
        status: reqwest::StatusCode,
        retryable: bool,
        body: String,
    },
}

impl JevError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Transport(error) | Self::Decode(error) if error.is_timeout())
    }

    pub fn is_decode(&self) -> bool {
        matches!(self, Self::Decode(_))
    }

    pub fn status(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Status-based hint for 429 and 5xx, not permission to replay a POST.
    /// Service-side idempotency and billing must be confirmed separately.
    /// The SDK does not retry automatically.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::HttpStatus {
                retryable: true,
                ..
            }
        )
    }

    pub fn error_body(&self) -> Option<&str> {
        match self {
            Self::HttpStatus { body, .. } => Some(body),
            _ => None,
        }
    }
}

impl std::fmt::Debug for JevError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildError => f.write_str("BuildError"),
            Self::Validation(reason) => f.debug_tuple("Validation").field(reason).finish(),
            Self::MissingApiKey => f.write_str("MissingApiKey"),
            Self::InvalidApiKey => f.write_str("InvalidApiKey"),
            Self::InvalidUrl => f.write_str("InvalidUrl"),
            Self::InvalidProxy => f.write_str("InvalidProxy"),
            Self::ClientBuild(_) => f.write_str("ClientBuild([REDACTED])"),
            Self::Transport(error) => f
                .debug_struct("Transport")
                .field("timeout", &error.is_timeout())
                .finish(),
            Self::Decode(error) => f
                .debug_struct("Decode")
                .field("timeout", &error.is_timeout())
                .finish(),
            Self::HttpStatus {
                status, retryable, ..
            } => f
                .debug_struct("HttpStatus")
                .field("status", status)
                .field("retryable", retryable)
                .field("body", &"[REDACTED]")
                .finish(),
        }
    }
}

impl std::fmt::Display for JevError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug already omits HTTP bodies and confidential configuration values.
        write!(f, "{self:?}")
    }
}

impl std::error::Error for JevError {}
