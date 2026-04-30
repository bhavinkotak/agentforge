use serde::{Deserialize, Serialize};

/// Token usage for a single trace or model call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }
}

/// USD cost breakdown for a trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Estimated total cost in USD.
    pub total_usd: f64,
    /// Cost attributed to input tokens.
    pub input_usd: f64,
    /// Cost attributed to output tokens.
    pub output_usd: f64,
    /// Model that generated this cost.
    pub model: String,
    /// Provider (openai, anthropic, etc.)
    pub provider: String,
}

/// A cost optimization recommendation: downgrade a model for a scenario subset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecommendation {
    /// Current model in use.
    pub current_model: String,
    /// Recommended cheaper model.
    pub recommended_model: String,
    /// Estimated savings per 1000 scenarios (USD).
    pub estimated_savings_usd: f64,
    /// Fraction of scenarios where the cheaper model scored equivalently.
    pub equivalent_score_fraction: f64,
    /// Aggregate score of recommended model on the test scenarios.
    pub candidate_aggregate_score: f64,
    /// Aggregate score of current model on the same scenarios.
    pub current_aggregate_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_total() {
        let u = TokenUsage::new(100, 50);
        assert_eq!(u.total_tokens, 150);
    }
}
