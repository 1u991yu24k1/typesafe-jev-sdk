use std::collections::HashMap;

use super::error::JevError;
use super::question::Question;
use super::request::JevRequest;

#[derive(Debug, Default)]
pub struct JevRequestBuilder {
    model: Option<String>,
    state: Option<String>,
    questions: HashMap<String, Question>,
}

impl JevRequestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(mut self, model_name: impl Into<String>) -> Self {
        self.model = Some(model_name.into());
        self
    }

    pub fn state(mut self, state_expr: impl Into<String>) -> Self {
        self.state = Some(state_expr.into());
        self
    }

    /// Adds a question, replacing any question with the same key.
    pub fn question(mut self, qkey: impl Into<String>, qval: Question) -> Self {
        self.questions.insert(qkey.into(), qval);
        self
    }

    pub fn build(self) -> Result<JevRequest, JevError> {
        let state = self.state.ok_or(JevError::BuildError)?;
        if self.questions.is_empty() {
            return Err(JevError::BuildError);
        }

        Ok(JevRequest {
            // Allocate the fallback only for a valid request without a model.
            model: self.model.unwrap_or_else(|| "jev-latest".to_owned()),
            state,
            questions: self.questions,
        })
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    fn question(instructions: &str) -> Question {
        Question::Noul {
            instructions: instructions.to_owned(),
            criteria: None,
        }
    }

    #[test]
    fn defaults_to_latest_model() {
        let request = JevRequestBuilder::new()
            .state("state")
            .question("ready", question("Ready?"))
            .build()
            .unwrap();

        assert_eq!(request.model, "jev-latest");
        assert_eq!(request.state, "state");
        assert_eq!(request.questions.len(), 1);
        assert_eq!(request.questions["ready"], question("Ready?"));
    }

    #[test]
    fn preserves_custom_model_and_all_questions() {
        let request = JevRequestBuilder::default()
            .model("custom-model")
            .state("状態")
            .question("first", Question::choice("Choose", vec![("a", "A")]))
            .question("second", Question::score("Score", vec!["low", "high"]))
            .question("third", question("Ready?"))
            .build()
            .unwrap();

        assert_eq!(request.model, "custom-model");
        assert_eq!(request.state, "状態");
        assert_eq!(request.questions.len(), 3);
    }

    #[test]
    fn duplicate_keys_replace_previous_questions() {
        let request = JevRequestBuilder::new()
            .state("state")
            .question("ready", question("Old"))
            .question("ready", question("New"))
            .build()
            .unwrap();

        assert_eq!(request.questions.len(), 1);
        assert_eq!(request.questions["ready"], question("New"));
    }

    #[test]
    fn last_model_and_state_win() {
        let request = JevRequestBuilder::new()
            .model("old")
            .model("new")
            .state("old")
            .state("new")
            .question("ready", question("Ready?"))
            .build()
            .unwrap();

        assert_eq!(request.model, "new");
        assert_eq!(request.state, "new");
    }

    #[test]
    fn missing_state_is_a_build_error() {
        let result = JevRequestBuilder::new()
            .question("ready", question("Ready?"))
            .build();
        assert!(matches!(result, Err(JevError::BuildError)));
    }

    #[test]
    fn missing_questions_is_a_build_error() {
        let result = JevRequestBuilder::new().state("state").build();
        assert!(matches!(result, Err(JevError::BuildError)));
        assert!(matches!(
            JevRequestBuilder::new().build(),
            Err(JevError::BuildError)
        ));
    }

    #[test]
    fn empty_strings_remain_valid_for_compatibility() {
        let request = JevRequestBuilder::new()
            .model("")
            .state("")
            .question("", question(""))
            .build()
            .unwrap();

        assert_eq!(request.model, "");
        assert_eq!(request.state, "");
        assert_eq!(request.questions[""], question(""));
    }
}
