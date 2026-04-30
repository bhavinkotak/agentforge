use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported public benchmark suites.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkSuite {
    Gaia,
    AgentBench,
    WebArena,
}

impl std::fmt::Display for BenchmarkSuite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchmarkSuite::Gaia => write!(f, "gaia"),
            BenchmarkSuite::AgentBench => write!(f, "agentbench"),
            BenchmarkSuite::WebArena => write!(f, "webarena"),
        }
    }
}

/// A single task loaded from a benchmark dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub suite: BenchmarkSuite,
    /// Task difficulty level as defined by the benchmark (e.g. GAIA levels 1/2/3).
    pub difficulty_level: Option<u8>,
    /// Natural language task description.
    pub question: String,
    /// Expected final answer.
    pub expected_answer: Option<String>,
    /// Any file attachments or context references.
    pub context_files: Vec<String>,
}

/// Result of running an agent on a benchmark task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub task_id: String,
    pub suite: BenchmarkSuite,
    pub agent_answer: Option<String>,
    pub correct: bool,
    pub score: f64,
    pub latency_ms: u64,
    pub token_cost_usd: f64,
}

/// Aggregated benchmark run for an agent against one suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub suite: BenchmarkSuite,
    pub total_tasks: u32,
    pub correct: u32,
    pub accuracy: f64,
    /// Percentile rank compared to published baseline (0.0–100.0).
    pub percentile_rank: Option<f64>,
    pub results: Vec<BenchmarkResult>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Published baseline accuracy for reference comparison.
#[derive(Debug, Clone)]
pub struct BenchmarkBaseline {
    pub suite: BenchmarkSuite,
    /// Baseline model name (e.g. "GPT-4o (OpenAI, April 2025)").
    pub model_name: String,
    pub accuracy: f64,
}

/// Hardcoded published baselines for percentile calculation.
pub fn published_baselines() -> Vec<BenchmarkBaseline> {
    vec![
        BenchmarkBaseline {
            suite: BenchmarkSuite::Gaia,
            model_name: "GPT-4o (OpenAI, 2025)".into(),
            accuracy: 0.53,
        },
        BenchmarkBaseline {
            suite: BenchmarkSuite::Gaia,
            model_name: "Claude 3.5 Sonnet (Anthropic, 2025)".into(),
            accuracy: 0.49,
        },
        BenchmarkBaseline {
            suite: BenchmarkSuite::AgentBench,
            model_name: "GPT-4 (OpenAI, 2024)".into(),
            accuracy: 0.45,
        },
        BenchmarkBaseline {
            suite: BenchmarkSuite::WebArena,
            model_name: "GPT-4o (OpenAI, 2025)".into(),
            accuracy: 0.39,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_display() {
        assert_eq!(BenchmarkSuite::Gaia.to_string(), "gaia");
        assert_eq!(BenchmarkSuite::AgentBench.to_string(), "agentbench");
        assert_eq!(BenchmarkSuite::WebArena.to_string(), "webarena");
    }

    #[test]
    fn baselines_non_empty() {
        assert!(!published_baselines().is_empty());
    }
}
