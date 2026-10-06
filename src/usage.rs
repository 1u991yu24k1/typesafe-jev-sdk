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

    pub fn input_tokens(&self) -> u64 {
        self.input_tokens
    }

    pub fn output_tokens(&self) -> u64 {
        self.output_tokens
    }

    /// Exact total, or `None` if the sum exceeds `u64::MAX`.
    pub fn checked_tokens(&self) -> Option<u64> {
        self.input_tokens.checked_add(self.output_tokens)
    }

    /// Compatibility accessor. Saturates on overflow; use `checked_tokens` when exactness matters.
    pub fn tokens(&self) -> u64 {
        self.checked_tokens().unwrap_or(u64::MAX)
    }

    /// Add counts from another response without losing either raw component.
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        Some(Self {
            input_tokens: self.input_tokens.checked_add(other.input_tokens)?,
            output_tokens: self.output_tokens.checked_add(other.output_tokens)?,
        })
    }

    /// Returns whether either token count is strictly greater than its budget.
    /// Counts equal to their limits are within budget.
    pub fn exceeds_budget(&self, budget: &Self) -> bool {
        self.input_tokens > budget.input_tokens || self.output_tokens > budget.output_tokens
    }
}
