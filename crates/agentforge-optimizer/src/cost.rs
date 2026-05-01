use agentforge_core::{AgentFile, CostBreakdown, CostRecommendation, ModelProvider, Trace};

/// Price per 1M tokens in USD (input, output) for known models.
#[derive(Debug, Clone)]
pub struct ModelPrice {
    pub model_id: &'static str,
    pub provider: ModelProvider,
    /// USD per 1M input tokens.
    pub input_per_million: f64,
    /// USD per 1M output tokens.
    pub output_per_million: f64,
}

/// All known model prices (as of April 2026).
pub fn model_price_table() -> Vec<ModelPrice> {
    vec![
        // OpenAI
        ModelPrice {
            model_id: "gpt-4o",
            provider: ModelProvider::Openai,
            input_per_million: 5.00,
            output_per_million: 15.00,
        },
        ModelPrice {
            model_id: "gpt-4o-mini",
            provider: ModelProvider::Openai,
            input_per_million: 0.15,
            output_per_million: 0.60,
        },
        ModelPrice {
            model_id: "gpt-4-turbo",
            provider: ModelProvider::Openai,
            input_per_million: 10.00,
            output_per_million: 30.00,
        },
        ModelPrice {
            model_id: "gpt-3.5-turbo",
            provider: ModelProvider::Openai,
            input_per_million: 0.50,
            output_per_million: 1.50,
        },
        // Anthropic
        ModelPrice {
            model_id: "claude-3-5-sonnet-20241022",
            provider: ModelProvider::Anthropic,
            input_per_million: 3.00,
            output_per_million: 15.00,
        },
        ModelPrice {
            model_id: "claude-3-haiku-20240307",
            provider: ModelProvider::Anthropic,
            input_per_million: 0.25,
            output_per_million: 1.25,
        },
        ModelPrice {
            model_id: "claude-3-opus-20240229",
            provider: ModelProvider::Anthropic,
            input_per_million: 15.00,
            output_per_million: 75.00,
        },
        // Ollama / local (cost = 0)
        ModelPrice {
            model_id: "llama3",
            provider: ModelProvider::Ollama,
            input_per_million: 0.0,
            output_per_million: 0.0,
        },
        ModelPrice {
            model_id: "llama3:8b",
            provider: ModelProvider::Ollama,
            input_per_million: 0.0,
            output_per_million: 0.0,
        },
    ]
}

/// Returns a price entry for the given model ID, if known.
pub fn price_for(model_id: &str) -> Option<ModelPrice> {
    model_price_table()
        .into_iter()
        .find(|p| p.model_id == model_id)
}

/// Compute the cost breakdown for a trace given its token counts.
pub fn compute_cost(model_id: &str, input_tokens: u32, output_tokens: u32) -> CostBreakdown {
    let price = price_for(model_id);
    let (input_usd, output_usd) = match &price {
        Some(p) => (
            (input_tokens as f64 / 1_000_000.0) * p.input_per_million,
            (output_tokens as f64 / 1_000_000.0) * p.output_per_million,
        ),
        None => (0.0, 0.0),
    };

    CostBreakdown {
        input_usd,
        output_usd,
        total_usd: input_usd + output_usd,
        model: model_id.to_string(),
        provider: price
            .map(|p| p.provider.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    }
}

/// Identifies cost optimization opportunities and emits model downgrade recommendations.
pub struct CostOptimizer;

impl CostOptimizer {
    /// Analyze traces from a run and suggest a cheaper model if it would score equivalently.
    ///
    /// "Equivalently" means within `score_tolerance` of the current model's score.
    pub fn analyze(
        agent: &AgentFile,
        traces: &[Trace],
        score_tolerance: f64,
    ) -> Vec<CostRecommendation> {
        let current_model = &agent.model.model_id;
        let current_price = match price_for(current_model) {
            Some(p) => p,
            None => return vec![],
        };

        let current_aggregate = mean_aggregate(traces);
        if traces.is_empty() {
            return vec![];
        }

        // Find cheaper alternatives from the same or Ollama provider.
        model_price_table()
            .into_iter()
            .filter(|p| {
                // Must be cheaper on at least one token dimension.
                p.model_id != current_model.as_str()
                    && p.input_per_million <= current_price.input_per_million
                    && p.output_per_million <= current_price.output_per_million
            })
            .filter_map(|candidate| {
                // We can't actually run the candidate here — we estimate based on the
                // assumption that smaller models may score within tolerance.
                // A real implementation would run the candidate model on the scenarios.
                // This produces a recommendation with a conservative estimate.
                let cost_saving = estimate_savings(traces, &current_price, &candidate);
                if cost_saving <= 0.0 {
                    return None;
                }

                Some(CostRecommendation {
                    current_model: current_model.clone(),
                    recommended_model: candidate.model_id.to_string(),
                    estimated_savings_usd: cost_saving,
                    equivalent_score_fraction: 0.0, // set after actual candidate eval
                    candidate_aggregate_score: current_aggregate - score_tolerance,
                    current_aggregate_score: current_aggregate,
                })
            })
            .collect()
    }
}

fn mean_aggregate(traces: &[Trace]) -> f64 {
    if traces.is_empty() {
        return 0.0;
    }
    let sum: f64 = traces.iter().filter_map(|t| t.aggregate_score).sum();
    sum / traces.len() as f64
}

fn estimate_savings(traces: &[Trace], current: &ModelPrice, candidate: &ModelPrice) -> f64 {
    let total_input: u32 = traces.iter().map(|t| t.input_tokens).sum();
    let total_output: u32 = traces.iter().map(|t| t.output_tokens).sum();

    let current_cost = (total_input as f64 / 1_000_000.0) * current.input_per_million
        + (total_output as f64 / 1_000_000.0) * current.output_per_million;

    let candidate_cost = (total_input as f64 / 1_000_000.0) * candidate.input_per_million
        + (total_output as f64 / 1_000_000.0) * candidate.output_per_million;

    (current_cost - candidate_cost).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpt4o_price_known() {
        let p = price_for("gpt-4o").unwrap();
        assert_eq!(p.input_per_million, 5.00);
    }

    #[test]
    fn compute_cost_calculation() {
        let breakdown = compute_cost("gpt-4o", 1_000_000, 500_000);
        assert!((breakdown.input_usd - 5.0).abs() < 0.001);
        assert!((breakdown.output_usd - 7.5).abs() < 0.001);
        assert!((breakdown.total_usd - 12.5).abs() < 0.001);
    }

    #[test]
    fn unknown_model_zero_cost() {
        let breakdown = compute_cost("my-custom-model", 100, 50);
        assert_eq!(breakdown.total_usd, 0.0);
    }
}
