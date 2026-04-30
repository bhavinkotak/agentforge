use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a shadow (online eval) run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShadowRunStatus {
    Pending,
    Running,
    Complete,
    Error,
}

impl std::fmt::Display for ShadowRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadowRunStatus::Pending => write!(f, "pending"),
            ShadowRunStatus::Running => write!(f, "running"),
            ShadowRunStatus::Complete => write!(f, "complete"),
            ShadowRunStatus::Error => write!(f, "error"),
        }
    }
}

/// The outcome of comparing champion vs. candidate on a single dimension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DimensionOutcome {
    Win,
    Loss,
    Tie,
}

/// Per-dimension comparison of champion vs. candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionComparison {
    pub dimension: String,
    pub champion_score: f64,
    pub candidate_score: f64,
    pub outcome: DimensionOutcome,
    pub delta: f64,
}

/// Aggregated shadow run comparison report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowComparison {
    pub run_id: Uuid,
    pub champion_agent_id: Uuid,
    pub candidate_agent_id: Uuid,
    /// Fraction of traffic routed to the candidate (0.0–1.0).
    pub traffic_fraction: f64,
    pub total_requests: u32,
    pub champion_aggregate_score: f64,
    pub candidate_aggregate_score: f64,
    pub aggregate_delta: f64,
    pub per_dimension: Vec<DimensionComparison>,
    /// Whether the candidate won overall (positive delta + all regression gates pass).
    pub candidate_wins: bool,
    pub compared_at: DateTime<Utc>,
}

/// Persisted record of a shadow run configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowRun {
    pub id: Uuid,
    pub champion_agent_id: Uuid,
    pub candidate_agent_id: Uuid,
    /// Percentage of traffic sent to candidate (0–100).
    pub traffic_percent: u8,
    pub status: ShadowRunStatus,
    pub comparison: Option<ShadowComparison>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display() {
        assert_eq!(ShadowRunStatus::Pending.to_string(), "pending");
        assert_eq!(ShadowRunStatus::Complete.to_string(), "complete");
    }
}
