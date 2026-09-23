use std::collections::HashMap;
use super::question::Question;
use super::request::JevRequest;
use super::error::JevError;

#[derive(Debug)]
pub struct JevRequestBuilder {
    model: Option<String>,
    state: Option<String>,
    question: Option<HashMap<String, Question>>
}

impl JevRequestBuilder {
    pub fn new() -> Self {
        Self { model: None, state: None, question: None }
    }

    pub fn model(mut self, model_name: impl Into<String>) -> Self {
        self.model = Some(model_name.into());
        self 
    }

    pub fn state(mut self, state_expr: impl Into<String>) -> Self {
        self.state = Some(state_expr.into());
        self
    }

    /// question をセット. 
    pub fn question(mut self, qkey: impl Into<String>, qval: Question) -> Self {
        let qkey = qkey.into();
        if self.question.is_none() {
            self.question = Some(HashMap::new());
        }
        if let Some(q) = &mut self.question {
            q.insert(qkey, qval);
        }
        self
    }

    pub fn build(self) -> Result<JevRequest, JevError> {
        let model = 
            self.model.unwrap_or("jev-latest".to_string());

        let state = 
            self.state.ok_or(JevError::BuildError)?;

        let questions = 
            self.question
            .and_then(|q| { if q.len() > 0 { Some(q) } else { None } } )
            .ok_or(JevError::BuildError)?;

        Ok(JevRequest { model, state, questions })
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn test_build_choise() {
        let builder = JevRequestBuilder::new();

        let req = 
            builder
            .model("jev-latest")
            .state("プレイヤーのHPは20%。敵が近くに3体いる。\n回復アイテムを1個持っている。")
            .question(
                "next_action", 
                Question::choice("次に取る行動は?",
                    vec![
                        ("heal",    "回復アイテムを使ってHPを回復する"),
                        ("retreat", "敵から距離を取って退避する"),
                        ("attack",  "近くの敵を攻撃する"),
                    ]
                )
            )
            .build();
        
        assert!(req.is_ok());
    }

    #[test]
    fn test_build_noul() {

    }
}