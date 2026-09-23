use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Answer {
    Choice { 
        choice: String /* Request 次第 */, 
        confidence: f64,
        probabilities: HashMap<String, f64>
    },

    Score {
        score: f64,
        confidence: f64,
        legend: HashMap<String, String>,
        probabilities: HashMap<String, f64>,
    },
    
    Noul { 
        noul: f64
    }
}

#[cfg(test)]
mod answer_tests {

    use super::*;
    
    /// Noul Answer の シリアライズ, 逆シリアライズテスト. 
    #[test]
    fn test_serde_noul() {
        let v = 
            r#"
            {
                "type": "noul",
                "noul": 0.82
            }
            "#;
        let x = serde_json::from_str::<Answer>(&v).unwrap();
       
        assert_eq!(
            x, 
            Answer::Noul { noul: 0.82_f64 }
        ); 

        assert!(serde_json::to_string(&x).is_ok())
    }


    /// Score Answer の シリアライズ, 逆シリアライズテスト. 
    #[test]
    fn test_serde_score() {
        let v = 
            r#"
            {
                "type": "score",
                "score": 2.0,
                "confidence": 1.0,
                "legend": {
                    "0": "低：差し迫った脅威がなく、安全に行動できる",
                    "1": "中：脅威があり、注意して行動する必要がある",
                    "2": "高：倒される危険が高く、直ちに対処する必要がある"
                },
                "probabilities": {
                    "0": 0.0,
                    "1": 0.0,
                    "2": 1.0
                }
            }
            "#;
        let x = serde_json::from_str::<Answer>(&v);
        
        assert!(x.is_ok(), "{:#?}", x.err());

        let x = x.unwrap();
        assert_eq!(
            x, 
            Answer::Score {
                score: 2.0_f64,
                confidence: 1.0_f64,
                legend: HashMap::from_iter(
                    [ 
                        ("0", "低：差し迫った脅威がなく、安全に行動できる"), 
                        ("1", "中：脅威があり、注意して行動する必要がある"),
                        ("2", "高：倒される危険が高く、直ちに対処する必要がある")
                    ]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                ),
                probabilities: HashMap::from_iter(
                    [
                        ("0", 0.0_f64),
                        ("1", 0.0_f64),
                        ("2", 1.0_f64),
                    ]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                ),
            }
        ); 
    }

    /// Score Answer の シリアライズ, 逆シリアライズテスト. 
    #[test]
    fn test_serde_choise() {
        let v = 
            r#"
            {
                "type": "choice",
                "choice": "heal",
                "confidence": 0.64,
                "probabilities": {
                    "heal": 0.76,
                    "attack": 0.0,
                    "retreat": 0.24
                }
            }
            "#;
        
        let x = serde_json::from_str::<Answer>(&v);

        assert!(x.is_ok(), "{:#?}", x.err());

        let x = x.unwrap();
        assert_eq!(
            x, 
            Answer::Choice {
                choice: String::from("heal"), 
                confidence: 0.64_f64,
                probabilities: HashMap::from_iter(
                    [
                        ("heal",    0.76_f64),
                        ("attack",  0.0_f64),
                        ("retreat", 0.24_f64),
                    ]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                )
            }
        );
    }
}