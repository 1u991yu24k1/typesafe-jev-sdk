use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::model::Question;

#[derive(Deserialize, Serialize, Debug)]
pub struct JevRequest {
    pub model: String,
    pub state: String,
    pub questions: HashMap<String, Question>,
}

impl JevRequest {
    pub fn new(model: impl Into<String>, state: impl Into<String>) -> Self {
        Self { 
            state: state.into(), 
            model: model.into(), 
            questions: HashMap::<String, Question>::new()
        }
    }

    pub fn add_question(&mut self, identifier: impl Into<String>, question: Question) {
        self.questions.insert(identifier.into(), question);
    }
}

#[cfg(test)]
mod request_body_tests {
    use super::*;
    #[test]
    fn test_request_body() {
        let body = 
            r#"
            {
                "state": "プレイヤーのHPは20%。敵が近くに3体いる。\n回復アイテムを1個持っている。",
                "model": "jev-latest",
                "questions": {
                    "next_action": {
                        "type": "choice",
                        "instructions": "次に取る行動は？",
                        "criteria": {
                          "heal": "回復アイテムを使ってHPを回復する",
                          "retreat": "敵から距離を取って退避する",
                          "attack": "近くの敵を攻撃する"
                        }
                                },
                    "danger_level": { 
                        "type": "score",
                        "instructions": "現在の危険度は？",
                        "criteria": [
                          "低：差し迫った脅威がなく、安全に行動できる",
                          "中：脅威があり、注意して行動する必要がある",
                          "高：倒される危険が高く、直ちに対処する必要がある"
                        ]
                    },
                    "need_healing": {
                        "type": "noul",
                        "instructions": "今すぐ回復する必要がある？",
                        "criteria": {
                          "true": "今すぐ回復する必要がある",
                          "false": "今すぐ回復する必要はない"
                        }
                    }
                }
            }
            "#; 

        let x = serde_json::from_str::<JevRequest>(&body).unwrap();
        println!("{:}", serde_json::to_string(&x).unwrap());        
    }
}


