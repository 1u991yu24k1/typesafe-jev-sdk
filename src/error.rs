pub enum JevError {
    BuildError,
    Validation(&'static str),
    MissingApiKey,
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
        matches!(self, Self::Transport(error) if error.is_timeout())
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
            Self::InvalidUrl => f.write_str("InvalidUrl"),
            Self::InvalidProxy => f.write_str("InvalidProxy"),
            Self::ClientBuild(error) => f.debug_tuple("ClientBuild").field(error).finish(),
            Self::Transport(error) => f.debug_tuple("Transport").field(error).finish(),
            Self::Decode(error) => f.debug_tuple("Decode").field(error).finish(),
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
