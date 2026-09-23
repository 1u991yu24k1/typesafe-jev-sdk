use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::usage::Usage;
use super::answer::Answer;

#[derive(Debug, Deserialize, Serialize)]
pub struct JevResponse {
    pub model: String,
    pub usage: Usage,
    pub answers: HashMap<String, Answer> 
}
