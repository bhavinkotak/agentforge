use agentforge_core::{DimensionComparison, DimensionOutcome, ShadowComparison, Trace};
use crate::shadow::ShadowRunOutput;
use chrono::Utc;
use uuid::Uuid;

/// Aggregated comparison report between champion and candidate.
pub struct ComparisonReport {
    pub comparison: ShadowComparison,
}

/// Compares champion vs. candidate traces and produces a `ShadowComparison`.
pub struct TrafficComparator;

impl TrafficComparator {
    pub fn compare(
        output: &ShadowRunOutput,
        champion_agent_id: Uuid,
        candidate_agent_id: Uuid,
        traffic_fraction: f64,
    ) -> ComparisonReport {
        let champ_agg = mean_aggregate(&output.champion_traces);
        let cand_agg = mean_aggregate(&output.candidate_traces);
        let delta = cand_agg - champ_agg;

        let per_dimension = compare_dimensions(&output.champion_traces, &output.candidate_traces);

        let candidate_wins = delta > 0.0
            && regression_gate_passes(&output.champion_traces, &output.candidate_traces);

        ComparisonReport {
            comparison: ShadowComparison {
                run_id: output.run_id,
                champion_agent_id,
                candidate_agent_id,
                traffic_fraction,
                total_requests: output.shared_scenarios.len() as u32,
                champion_aggregate_score: champ_agg,
                candidate_aggregate_score: cand_agg,
                aggregate_delta: delta,
                per_dimension,
                candidate_wins,
                compared_at: Utc::now(),
            },
        }
    }
}

fn mean_aggregate(traces: &[Trace]) -> f64 {
    if traces.is_empty() {
        return 0.0;
    }
    let sum: f64 = traces.iter().filter_map(|t| t.aggregate_score).sum();
    sum / traces.len() as f64
}

fn compare_dimensions(champion: &[Trace], candidate: &[Trace]) -> Vec<DimensionComparison> {
    macro_rules! dim {
        ($name:expr, $field:ident) => {{
            let champ_avg = dimension_avg(champion, |s| s.$field);
            let cand_avg = dimension_avg(candidate, |s| s.$field);
            let delta = cand_avg - champ_avg;
            DimensionComparison {
                dimension: $name.to_string(),
                champion_score: champ_avg,
                candidate_score: cand_avg,
                outcome: if delta > 0.005 {
                    DimensionOutcome::Win
                } else if delta < -0.005 {
                    DimensionOutcome::Loss
                } else {
                    DimensionOutcome::Tie
                },
                delta,
            }
        }};
    }

    vec![
        dim!("task_completion", task_completion),
        dim!("tool_selection", tool_selection),
        dim!("argument_correctness", argument_correctness),
        dim!("schema_compliance", schema_compliance),
        dim!("instruction_adherence", instruction_adherence),
        dim!("path_efficiency", path_efficiency),
    ]
}

fn dimension_avg<F: Fn(&agentforge_core::DimensionScores) -> f64>(
    traces: &[Trace],
    accessor: F,
) -> f64 {
    let scored: Vec<f64> = traces
        .iter()
        .filter_map(|t| t.scores.as_ref())
        .map(&accessor)
        .collect();

    if scored.is_empty() {
        return 0.0;
    }
    scored.iter().sum::<f64>() / scored.len() as f64
}

/// Regression gate: candidate must pass at least 99% of scenarios champion passes.
fn regression_gate_passes(champion: &[Trace], candidate: &[Trace]) -> bool {
    use agentforge_core::TraceStatus;

    let champion_pass_ids: std::collections::HashSet<_> = champion
        .iter()
        .filter(|t| t.status == TraceStatus::Pass)
        .map(|t| t.scenario_id)
        .collect();

    if champion_pass_ids.is_empty() {
        return true;
    }

    let candidate_pass_ids: std::collections::HashSet<_> = candidate
        .iter()
        .filter(|t| t.status == TraceStatus::Pass)
        .map(|t| t.scenario_id)
        .collect();

    let regression_count = champion_pass_ids
        .iter()
        .filter(|id| !candidate_pass_ids.contains(*id))
        .count();

    let regression_rate = regression_count as f64 / champion_pass_ids.len() as f64;
    regression_rate <= 0.01
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_aggregate_empty() {
        assert_eq!(mean_aggregate(&[]), 0.0);
    }
}
