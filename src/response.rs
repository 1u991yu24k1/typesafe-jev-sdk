use super::answer::Answer;
use super::question::Question;
use super::request::JevRequest;
use super::usage::Usage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::HeaderMap;

/// HTTP metadata kept separate from the API's JSON response contract.
///
/// Header values may contain credentials or other confidential information.
/// They are available explicitly through `headers`, but omitted from `Debug`.
pub struct ResponseMetadata {
    status: StatusCode,
    headers: HeaderMap,
    elapsed: Duration,
}

impl ResponseMetadata {
    pub(crate) fn new(status: StatusCode, headers: HeaderMap, elapsed: Duration) -> Self {
        Self {
            status,
            headers,
            elapsed,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the received headers. Do not log these without redaction.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Time from request construction through complete body reception and JSON decoding.
    /// Does not include any caller-side validation or processing.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Read an ID from an explicitly named response header.
    ///
    /// No JEV-specific header name is assumed. Returns `None` for a missing
    /// header or a value that cannot be represented as an HTTP header string.
    /// This does not fabricate or validate service-side request IDs.
    pub fn request_id(&self, header_name: &str) -> Option<&str> {
        self.headers.get(header_name)?.to_str().ok()
    }
}

impl fmt::Debug for ResponseMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseMetadata")
            .field("status", &self.status)
            .field("headers", &"[REDACTED]")
            .field("elapsed", &self.elapsed)
            .finish()
    }
}

/// A decoded API response and its HTTP metadata. Not part of the wire format.
pub struct JevResponseWithMetadata {
    pub response: JevResponse,
    pub metadata: ResponseMetadata,
}

impl fmt::Debug for JevResponseWithMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JevResponseWithMetadata")
            .field("response", &"[REDACTED]")
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JevResponse {
    pub model: String,
    pub usage: Usage,
    pub answers: HashMap<String, Answer>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResponseValidationError {
    MissingAnswer(String),
    UnexpectedAnswer(String),
    MismatchedAnswerType(String),
    InvalidValue(String),
    UnknownChoice(String),
}

impl JevResponse {
    /// Optionally validate semantic consistency without changing raw JSON decoding.
    pub fn validate_against(&self, request: &JevRequest) -> Result<(), ResponseValidationError> {
        for (key, question) in &request.questions {
            let answer = self
                .answers
                .get(key)
                .ok_or_else(|| ResponseValidationError::MissingAnswer(key.to_owned()))?;
            match (question, answer) {
                (
                    Question::Choice { criteria, .. },
                    Answer::Choice {
                        choice,
                        confidence,
                        probabilities,
                    },
                ) => {
                    if !criteria.contains_key(choice) {
                        return Err(ResponseValidationError::UnknownChoice(key.to_owned()));
                    }
                    if !unit_interval(*confidence)
                        || probabilities.values().any(|p| !unit_interval(*p))
                    {
                        return Err(ResponseValidationError::InvalidValue(key.to_owned()));
                    }
                    if probabilities.keys().any(|id| !criteria.contains_key(id)) {
                        return Err(ResponseValidationError::UnknownChoice(key.to_owned()));
                    }
                }
                (
                    Question::Score { .. },
                    Answer::Score {
                        score,
                        confidence,
                        probabilities,
                        ..
                    },
                ) => {
                    if !score.is_finite()
                        || !unit_interval(*confidence)
                        || probabilities.values().any(|p| !unit_interval(*p))
                    {
                        return Err(ResponseValidationError::InvalidValue(key.to_owned()));
                    }
                }
                (Question::Noul { .. }, Answer::Noul { noul }) => {
                    if !unit_interval(*noul) {
                        return Err(ResponseValidationError::InvalidValue(key.to_owned()));
                    }
                }
                _ => {
                    return Err(ResponseValidationError::MismatchedAnswerType(
                        key.to_owned(),
                    ));
                }
            }
        }
        if let Some(key) = self
            .answers
            .keys()
            .find(|key| !request.questions.contains_key(*key))
        {
            return Err(ResponseValidationError::UnexpectedAnswer(key.to_owned()));
        }
        Ok(())
    }
}

fn unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn non_text_request_id_is_not_interpreted_and_debug_is_redacted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test-id", HeaderValue::from_bytes(&[0x80]).unwrap());
        headers.insert(
            "set-cookie",
            HeaderValue::from_static("test-secret-not-real"),
        );
        let metadata = ResponseMetadata::new(StatusCode::OK, headers, Duration::ZERO);
        assert_eq!(metadata.request_id("x-test-id"), None);
        assert!(!format!("{metadata:?}").contains("test-secret-not-real"));
        assert_eq!(metadata.elapsed(), Duration::ZERO);
    }
}
