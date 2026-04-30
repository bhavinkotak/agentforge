use crate::{DimensionScores, FailureCluster};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A complete execution trace for a single scenario run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub id: Uuid,
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    pub status: TraceStatus,
    /// Ordered list of execution steps.
    pub steps: Vec<TraceStep>,
    pub final_output: Option<serde_json::Value>,
    pub scores: Option<DimensionScores>,
    pub aggregate_score: Option<f64>,
    pub failure_cluster: FailureCluster,
    pub failure_reason: Option<String>,
    pub review_needed: bool,
    pub llm_calls: u32,
    pub tool_invocations: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
    pub retry_count: u32,
    pub seed: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Pass,
    Fail,
    Error,
    ReviewNeeded,
}

impl std::fmt::Display for TraceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceStatus::Pass => write!(f, "pass"),
            TraceStatus::Fail => write!(f, "fail"),
            TraceStatus::Error => write!(f, "error"),
            TraceStatus::ReviewNeeded => write!(f, "review_needed"),
        }
    }
}

/// A single step in an execution trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceStep {
    LlmCall(LlmCallStep),
    ToolCall(ToolCallStep),
    ToolResult(ToolResultStep),
    AgentThought(AgentThoughtStep),
    FinalOutput(FinalOutputStep),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallStep {
    pub index: u32,
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    pub response: serde_json::Value,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStep {
    pub index: u32,
    pub tool_name: String,
    pub call_id: String,
    pub arguments: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultStep {
    pub index: u32,
    pub tool_name: String,
    pub call_id: String,
    pub result: serde_json::Value,
    pub is_error: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentThoughtStep {
    pub index: u32,
    pub thought: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalOutputStep {
    pub index: u32,
    pub output: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_status_display() {
        assert_eq!(TraceStatus::Pass.to_string(), "pass");
        assert_eq!(TraceStatus::ReviewNeeded.to_string(), "review_needed");
    }
}
