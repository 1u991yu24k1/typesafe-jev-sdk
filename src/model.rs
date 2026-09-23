use std::env;
use std::collections::HashMap;
use serde::{Deserialize, Serialize}; 

#[derive(Debug, Deserialize, Serialize)]
pub struct Usage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all= "snake_case")]
pub enum Question {
    Choice { instructions: String, criteria: HashMap<String, String> },
    Score  { instructions: String, criteria: Vec<String> },

    Noul   { 
        instructions: String, 

        // Noul の criteria はなくても良いらしい .
        // Boolean の境界条件を明確にしたい場合は, "true" / "false" でつける.  
        #[serde(skip_serializing_if = "Option::is_none")]
        criteria: Option<HashMap<String, String>> 
    }, 
}

impl Question {
    pub fn choice(
        inst: impl Into<String>, 
        criteria: Vec<(impl Into<String>, impl Into<String>)>
    ) -> Self {
        let instructions = inst.into();
        let criteria = 
            HashMap::from_iter(
                criteria
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
            );
        Self::Choice { instructions, criteria } 
    }

    pub fn score(
        inst: impl Into<String>, 
        criteria: Vec<impl Into<String>>
    ) -> Self {
        let instructions = inst.into();
        let criteria = 
            criteria
            .into_iter()
            .map(|x| x.into())
            .collect();

        Self::Score { instructions, criteria }
    }

    pub fn noul(
        inst: impl Into<String>, 
        criteria: Option<Vec<(impl Into<String>, impl Into<String>)>> 
    ) -> Self {
        let instructions = inst.into();
        let criteria = 
            criteria
            .map(|v| {
                v.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()
            });
        Self::Noul { instructions, criteria }
    }
}


#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Answer {
    Choice { 
        choice: String /* Request 次第 */, 
        confidence: f64,
        probabilities: HashMap<String, f32>
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
mod tests {
    use super::*;

    /// Noul Question の シリアライズ, 逆シリアライズテスト. 
    #[test]
    fn test_serde_noul_from_str() {
        let v = r#"
        {
            "type": "noul",
            "instructions": "今すぐ回復する必要がある?",
            "criteria": {
                "true": "今すぐ回復する必要がある",
                "false": "今すぐ回復する必要はない"
            }
        }
        "#;

        let instructions = String::from("今すぐ回復する必要がある?");
        let true_noul = "今すぐ回復する必要がある";
        let false_noul = "今すぐ回復する必要はない";

        let x = serde_json::from_str::<Question>(&v).unwrap();

        let mut criteria: HashMap<String, String> = HashMap::new();
        criteria.insert("true".to_string(), true_noul.to_string());
        criteria.insert("false".to_string(), false_noul.to_string());

        let criteria = Some(criteria);
        assert_eq!(
            x, 
            Question::Noul { instructions , criteria }
        );
    }

    /// Noul Question の シリアライズ, 逆シリアライズテスト. (criteria無し)
    #[test]
    fn test_serde_noul_without_criteria_from_str() {
        let v = 
            r#"
            {
                "type": "noul",
                "instructions": "今すぐ回復する必要がある?"
            }
            "#;

        let x = serde_json::from_str::<Question>(&v);
        assert!(x.is_ok(), "{:#?}", x.err());
        let x = x.unwrap();

        assert_eq!(
            x,
            Question::Noul { 
                instructions: String::from("今すぐ回復する必要がある?"),
                criteria: None,
            }
        );
        
    }

    /// Noul Answer の シリアライズ, 逆シリアライズテスト. 
    #[test]
    fn test_deserialize_noul_answer_from_str() {
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


    #[test]
    fn test_deserialize_score_from_str() {
        let v = r#"
        {
                        "type": "score",
                        "instructions": "現在の危険度は？",
                        "criteria": [
                          "低：差し迫った脅威がなく、安全に行動できる",
                          "中：脅威があり、注意して行動する必要がある",
                          "高：倒される危険が高く、直ちに対処する必要がある"
                        ]
                    } 
        "#;
        
        let x = 
            serde_json::from_str::<Question>(&v)
            .unwrap();
        
        let criteria: Vec<String> = 
            [
               "低：差し迫った脅威がなく、安全に行動できる",
               "中：脅威があり、注意して行動する必要がある",
               "高：倒される危険が高く、直ちに対処する必要がある"
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(); 

        assert_eq!(
            x, 
            Question::Score {
                instructions: "現在の危険度は？".into(),
                criteria,        
            }
        );
    }

    #[test]
    fn test_serde_score_response() {
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

    #[test]
    fn test_deserialize_choise_from_str() {
        let v = 
            r#"
            {
                "type": "choice",
                "instructions": "次に取る行動は?",
                "criteria": {
                    "heal": "回復アイテムを使ってHPを回復する",
                    "retreat": "敵から距離を取って退避する",
                    "attack": "近くの敵を攻撃する"
                }
            }
            "#;
        let x = 
            serde_json::from_str::<Question>(&v)
            .unwrap();
      
        let instructions = String::from("次に取る行動は?"); 

        let criteria: HashMap<String, String> = 
            HashMap::from_iter(
                [
                    ("heal"   , "回復アイテムを使ってHPを回復する"),
                    ("retreat", "敵から距離を取って退避する"), 
                    ("attack" , "近くの敵を攻撃する")
                ]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .by_ref()
        );

        assert_eq!(
            x, Question::Choice{ instructions, criteria }
        ) 
        
    }
}
