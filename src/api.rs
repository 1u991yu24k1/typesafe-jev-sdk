use std::time::Duration;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use reqwest::{Url, Client};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use super::model::{Question, Answer, Usage};


#[derive(Deserialize, Serialize, Debug)]
pub struct JevRequest {
    pub model: String,
    pub state: String,
    pub questions: HashMap<String, Question>,
}

impl JevRequest {
    pub fn new(
        model: impl Into<String>, 
        state: impl Into<String>, 
    ) -> Self {
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

#[derive(Debug)]
pub enum JevError {
    BuildError,

} 

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



#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseBody {
    pub model: String,
    pub usage: Usage,
    pub answers: HashMap<String, Answer> 
}


#[derive(Debug)]
pub struct TypeSafeClient {
    pub client: Client,
    pub url: Url,
    api_key: String
}

impl TypeSafeClient {
    pub fn new(timeout: Duration) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE,  HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT,        HeaderValue::from_static("application/json"));
        
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(timeout)
            .default_headers(headers)
            .https_only(true)
            .build()
            .unwrap();
       
        let url = 
            Url::parse(
                &std::env::var("TYPESAFE_API_BASE_URL")
                    .unwrap_or("https://api.typesafe.ai/v1/systemone".to_string())
            ).unwrap();


        let api_key = 
            std::env::var("TYPESAFE_API_KEY")
            .unwrap();

        Self { client, url, api_key } 
    }

    pub async fn system_one(&self, request: &JevRequest) -> Result<ResponseBody, reqwest::Error> {
        let resp = 
            self.client
            .post(self.url.as_str())
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?
            .json::<ResponseBody>()
            .await?;

        Ok(resp)
    }
}
