use agentforge_core::{MultiAgentScorecard, Trace};
use std::collections::HashMap;
use uuid::Uuid;

/// Full result of executing a graph against a set of scenarios.
pub struct GraphRunResult {
    pub graph_id: Uuid,
    /// Per-node traces keyed by node id.
    pub node_traces: HashMap<String, Vec<Trace>>,
    pub scorecard: MultiAgentScorecard,
}

impl GraphRunResult {
    /// Build from per-node traces. Scores each node and computes a composite.
    pub fn from_traces(graph_id: Uuid, node_traces: HashMap<String, Vec<Trace>>) -> Self {
        let node_scores: HashMap<String, f64> = node_traces
            .iter()
            .map(|(node_id, traces)| {
                let avg = if traces.is_empty() {
                    0.0
                } else {
                    traces.iter().filter_map(|t| t.aggregate_score).sum::<f64>()
                        / traces.len() as f64
                };
                (node_id.clone(), avg)
            })
            .collect();

        let composite_score = if node_scores.is_empty() {
            0.0
        } else {
            node_scores.values().sum::<f64>() / node_scores.len() as f64
        };

        let total_traces: usize = node_traces.values().map(|v| v.len()).sum();
        let passing: usize = node_traces
            .values()
            .flat_map(|v| v.iter())
            .filter(|t| t.aggregate_score.map(|s| s >= 0.5).unwrap_or(false))
            .count();

        let pass_rate = if total_traces == 0 {
            0.0
        } else {
            passing as f64 / total_traces as f64
        };

        GraphRunResult {
            graph_id,
            node_traces,
            scorecard: MultiAgentScorecard {
                graph_id,
                node_scores,
                composite_score,
                pass_rate,
            },
        }
    }
}
