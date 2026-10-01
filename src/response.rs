use super::answer::Answer;
use super::usage::Usage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct JevResponse {
    pub model: String,
    pub usage: Usage,
    pub answers: HashMap<String, Answer>,
}
