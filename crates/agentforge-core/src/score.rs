use serde::{Deserialize, Serialize};

/// Score result for a single dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub value: f64,
    /// Confidence that this score is correct (0.0–1.0).
    /// Scores below 0.5 confidence are flagged for human review.
    pub confidence: f64,
    pub method: ScoringMethod,
    pub rationale: Option<String>,
}

impl Default for DimensionScore {
    fn default() -> Self {
        Self {
            value: 0.0,
            confidence: 1.0,
            method: ScoringMethod::Deterministic,
            rationale: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScoringMethod {
    /// Deterministic check (schema validation, keyword match, etc.)
    Deterministic,
    /// LLM judge evaluation
    LlmJudge,
    /// Combination of both
    Hybrid,
    /// Heuristic approximation (no LLM, no hard rules)
    Heuristic,
}

/// Scorecard: the aggregated scoring result for a full eval run,
/// ready for display in the dashboard / CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    pub run_id: uuid::Uuid,
    pub agent_id: uuid::Uuid,
    pub agent_name: String,
    pub agent_version: String,
    pub aggregate_score: f64,
    pub pass_rate: f64,
    pub total_scenarios: u32,
    pub passed: u32,
    pub failed: u32,
    pub errors: u32,
    pub review_needed: u32,
    pub dimension_scores: crate::DimensionScores,
    pub failure_clusters: Vec<crate::FailureClusterSummary>,
    pub duration_seconds: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

/// A scorecard delta between two agent versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardDiff {
    pub version_a: String,
    pub version_b: String,
    pub aggregate_score_delta: f64,
    pub pass_rate_delta: f64,
    pub dimension_deltas: DimensionDeltas,
    pub regression_count: u32,
    pub improvement_count: u32,
    pub neutral_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionDeltas {
    pub task_completion: f64,
    pub tool_selection: f64,
    pub argument_correctness: f64,
    pub schema_compliance: f64,
    pub instruction_adherence: f64,
    pub path_efficiency: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoring_method_serde() {
        let json = serde_json::to_string(&ScoringMethod::LlmJudge).unwrap();
        assert_eq!(json, r#""llm_judge""#);
        let back: ScoringMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ScoringMethod::LlmJudge);
    }
}
