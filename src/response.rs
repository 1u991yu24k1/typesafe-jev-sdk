use super::answer::Answer;
use super::question::Question;
use super::request::JevRequest;
use super::usage::Usage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
