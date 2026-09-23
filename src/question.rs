use std::collections::HashMap;
use serde::{Deserialize, Serialize}; 


#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all= "snake_case")]
pub enum Question {
    Choice { 
        instructions: String, 
        criteria: HashMap<String, String> 
    },
    
    Score { 
        instructions: String, 
        criteria: Vec<String> 
    },

    Noul { 
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


#[cfg(test)]
mod question_tests {
    use super::*;

    /// Noul Question の シリアライズ, 逆シリアライズテスト. (criteria有り)
    #[test]
    fn test_serde_noul_with_criteria() {
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
    fn test_serde_noul_without_criteria() {
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

    #[test]
    fn test_serde_score() {
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
    fn test_serde_choise() {
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
