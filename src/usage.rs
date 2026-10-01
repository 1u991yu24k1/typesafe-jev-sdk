use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Usage {
    input_tokens: u64,
    output_tokens: u64,
}

impl Usage {
    pub fn as_budget(max_input: u64, max_output: u64) -> Self {
        Self {
            input_tokens: max_input,
            output_tokens: max_output,
        }
    }

    pub fn tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Returns whether either token count is strictly greater than its budget.
    /// Counts equal to their limits are within budget.
    pub fn exceeds_budget(&self, budget: &Self) -> bool {
        self.input_tokens > budget.input_tokens || self.output_tokens > budget.output_tokens
    }
}
