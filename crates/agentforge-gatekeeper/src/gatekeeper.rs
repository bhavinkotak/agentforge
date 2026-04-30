use agentforge_core::{Result, Scorecard, Trace, TraceStatus};
use uuid::Uuid;

/// Configuration for all promotion gates.
#[derive(Debug, Clone)]
pub struct GatekeeperConfig {
    /// Minimum improvement over champion aggregate score to pass (default: 0.03 = +3%)
    pub score_gate_delta: f64,
    /// Minimum fraction of champion-passing scenarios the challenger must also pass (default: 0.99)
    pub regression_gate_ratio: f64,
    /// Number of random seeds that must all pass before promotion (default: 3)
    pub stability_seeds: u32,
}

impl Default for GatekeeperConfig {
    fn default() -> Self {
        Self {
            score_gate_delta: std::env::var("AGENTFORGE_SCORE_GATE_DELTA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.03),
            regression_gate_ratio: std::env::var("AGENTFORGE_REGRESSION_GATE_RATIO")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.99),
            stability_seeds: std::env::var("AGENTFORGE_STABILITY_SEEDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
        }
    }
}

/// Pass/fail result for a single gate.
#[derive(Debug, Clone, PartialEq)]
pub struct GateResult {
    pub gate: GateKind,
    pub status: GateStatus,
    pub message: String,
    /// Numeric delta (actual vs threshold) for display
    pub delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GateKind {
    Score,
    Regression,
    Stability,
}

impl std::fmt::Display for GateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateKind::Score => write!(f, "Score Gate"),
            GateKind::Regression => write!(f, "Regression Gate"),
            GateKind::Stability => write!(f, "Stability Gate"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GateStatus {
    Pass,
    Fail,
    Waived, // No champion exists yet
}

/// The overall promotion decision.
#[derive(Debug, Clone)]
pub struct PromotionDecision {
    pub run_id: Uuid,
    pub agent_id: Uuid,
    pub approved: bool,
    pub gates: Vec<GateResult>,
    pub changelog: String,
}

/// The gatekeeper evaluates a challenger scorecard against the champion.
pub struct Gatekeeper {
    pub config: GatekeeperConfig,
}

impl Gatekeeper {
    pub fn new(config: GatekeeperConfig) -> Self {
        Self { config }
    }

    /// Evaluate all gates and return a promotion decision.
    ///
    /// `champion_scorecard`: None if no champion exists yet (first promotion is auto-approved).
    /// `challenger_scorecard`: The scorecard from the candidate run.
    /// `champion_passing_scenario_ids`: The set of scenario IDs the champion passed.
    /// `challenger_traces`: The challenger's traces (for regression analysis).
    /// `challenger_seed_scores`: Aggregate scores from each seed run (for stability).
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        &self,
        run_id: Uuid,
        agent_id: Uuid,
        champion_scorecard: Option<&Scorecard>,
        challenger_scorecard: &Scorecard,
        champion_passing_scenario_ids: &[Uuid],
        challenger_traces: &[Trace],
        challenger_seed_scores: &[f64],
    ) -> Result<PromotionDecision> {
        let mut gates = Vec::new();

        // 1. Score Gate
        let score_gate = self.check_score_gate(champion_scorecard, challenger_scorecard);
        let score_passed =
            score_gate.status == GateStatus::Pass || score_gate.status == GateStatus::Waived;
        gates.push(score_gate);

        // 2. Regression Gate
        let regression_gate =
            self.check_regression_gate(champion_passing_scenario_ids, challenger_traces);
        let regression_passed = regression_gate.status == GateStatus::Pass
            || regression_gate.status == GateStatus::Waived;
        gates.push(regression_gate);

        // 3. Stability Gate
        let stability_gate = self.check_stability_gate(challenger_seed_scores);
        let stability_passed = stability_gate.status == GateStatus::Pass
            || stability_gate.status == GateStatus::Waived;
        gates.push(stability_gate);

        let approved = score_passed && regression_passed && stability_passed;

        let changelog = build_changelog(champion_scorecard, challenger_scorecard, &gates);

        tracing::info!(
            run_id = %run_id,
            approved = approved,
            score_gate = %gates[0].status == GateStatus::Pass,
            regression_gate = %gates[1].status == GateStatus::Pass,
            stability_gate = %gates[2].status == GateStatus::Pass,
            "Gatekeeper evaluation complete"
        );

        if !approved {
            // Return gates info but surface the primary reason as an error type
            let failed_gates: Vec<String> = gates
                .iter()
                .filter(|g| g.status == GateStatus::Fail)
                .map(|g| format!("{}: {}", g.gate, g.message))
                .collect();

            return Ok(PromotionDecision {
                run_id,
                agent_id,
                approved: false,
                gates,
                changelog: format!(
                    "Promotion DENIED. Failed gates:\n{}",
                    failed_gates.join("\n")
                ),
            });
        }

        Ok(PromotionDecision {
            run_id,
            agent_id,
            approved: true,
            gates,
            changelog,
        })
    }

    fn check_score_gate(&self, champion: Option<&Scorecard>, challenger: &Scorecard) -> GateResult {
        let Some(champ) = champion else {
            return GateResult {
                gate: GateKind::Score,
                status: GateStatus::Waived,
                message: "No existing champion — score gate waived for first promotion".to_string(),
                delta: None,
            };
        };

        let delta = challenger.aggregate_score - champ.aggregate_score;
        let threshold = self.config.score_gate_delta;

        if delta >= threshold {
            GateResult {
                gate: GateKind::Score,
                status: GateStatus::Pass,
                message: format!(
                    "Challenger aggregate {:.3} vs champion {:.3} (+{:.3}, required +{:.3})",
                    challenger.aggregate_score, champ.aggregate_score, delta, threshold
                ),
                delta: Some(delta),
            }
        } else {
            GateResult {
                gate: GateKind::Score,
                status: GateStatus::Fail,
                message: format!(
                    "Score gate failed: delta {:.3} < required {:.3} (challenger {:.3} vs champion {:.3})",
                    delta, threshold, challenger.aggregate_score, champ.aggregate_score
                ),
                delta: Some(delta),
            }
        }
    }

    fn check_regression_gate(
        &self,
        champion_passing: &[Uuid],
        challenger_traces: &[Trace],
    ) -> GateResult {
        if champion_passing.is_empty() {
            return GateResult {
                gate: GateKind::Regression,
                status: GateStatus::Waived,
                message: "No champion traces — regression gate waived".to_string(),
                delta: None,
            };
        }

        // Build a set of scenario IDs the challenger passed
        let challenger_passed: std::collections::HashSet<Uuid> = challenger_traces
            .iter()
            .filter(|t| t.status == TraceStatus::Pass)
            .map(|t| t.scenario_id)
            .collect();

        let champion_total = champion_passing.len() as f64;
        let still_passing = champion_passing
            .iter()
            .filter(|id| challenger_passed.contains(id))
            .count() as f64;

        let retention_rate = still_passing / champion_total;
        let threshold = self.config.regression_gate_ratio;

        if retention_rate >= threshold {
            GateResult {
                gate: GateKind::Regression,
                status: GateStatus::Pass,
                message: format!(
                    "Challenger retains {:.1}% of champion-passing scenarios (>= {:.1}% required)",
                    retention_rate * 100.0,
                    threshold * 100.0
                ),
                delta: Some(retention_rate - threshold),
            }
        } else {
            let regressions = (champion_total - still_passing) as u32;
            GateResult {
                gate: GateKind::Regression,
                status: GateStatus::Fail,
                message: format!(
                    "Regression gate failed: {regressions} regressions detected. Retention {:.1}% < {:.1}% required",
                    retention_rate * 100.0,
                    threshold * 100.0
                ),
                delta: Some(retention_rate - threshold),
            }
        }
    }

    fn check_stability_gate(&self, seed_scores: &[f64]) -> GateResult {
        let required = self.config.stability_seeds as usize;

        if seed_scores.len() < required {
            return GateResult {
                gate: GateKind::Stability,
                status: GateStatus::Fail,
                message: format!(
                    "Stability gate failed: only {} seed run(s) provided, {} required",
                    seed_scores.len(),
                    required
                ),
                delta: None,
            };
        }

        // Check that variance is not too high (all seeds within 5% of each other)
        let min = seed_scores.iter().cloned().fold(f64::MAX, f64::min);
        let max = seed_scores.iter().cloned().fold(f64::MIN, f64::max);
        let variance = max - min;

        if variance > 0.05 {
            return GateResult {
                gate: GateKind::Stability,
                status: GateStatus::Fail,
                message: format!(
                    "Stability gate failed: score variance {:.3} across seeds (max allowed: 0.05)",
                    variance
                ),
                delta: Some(-variance),
            };
        }

        GateResult {
            gate: GateKind::Stability,
            status: GateStatus::Pass,
            message: format!(
                "{} seed runs passed with score variance {:.3} (< 0.05 required)",
                seed_scores.len(),
                variance
            ),
            delta: Some(0.05 - variance),
        }
    }
}

fn build_changelog(
    champion: Option<&Scorecard>,
    challenger: &Scorecard,
    gates: &[GateResult],
) -> String {
    let mut lines = vec![
        format!(
            "# Promotion: {} v{}",
            challenger.agent_name, challenger.agent_version
        ),
        String::new(),
        "## Score Summary".to_string(),
    ];

    if let Some(champ) = champion {
        let agg_delta = challenger.aggregate_score - champ.aggregate_score;
        lines.push(format!(
            "- Aggregate: {:.3} → {:.3} ({:+.3})",
            champ.aggregate_score, challenger.aggregate_score, agg_delta
        ));
        let d = &challenger.dimension_scores;
        let cd = &champ.dimension_scores;
        lines.push(format!(
            "- Task Completion: {:.3} → {:.3} ({:+.3})",
            cd.task_completion,
            d.task_completion,
            d.task_completion - cd.task_completion
        ));
        lines.push(format!(
            "- Tool Selection: {:.3} → {:.3} ({:+.3})",
            cd.tool_selection,
            d.tool_selection,
            d.tool_selection - cd.tool_selection
        ));
        lines.push(format!(
            "- Argument Correctness: {:.3} → {:.3} ({:+.3})",
            cd.argument_correctness,
            d.argument_correctness,
            d.argument_correctness - cd.argument_correctness
        ));
        lines.push(format!(
            "- Schema Compliance: {:.3} → {:.3} ({:+.3})",
            cd.schema_compliance,
            d.schema_compliance,
            d.schema_compliance - cd.schema_compliance
        ));
        lines.push(format!(
            "- Instruction Adherence: {:.3} → {:.3} ({:+.3})",
            cd.instruction_adherence,
            d.instruction_adherence,
            d.instruction_adherence - cd.instruction_adherence
        ));
    } else {
        lines.push(format!(
            "- First promotion — aggregate score: {:.3}",
            challenger.aggregate_score
        ));
    }

    lines.push(String::new());
    lines.push("## Gate Results".to_string());
    for gate in gates {
        let status = match &gate.status {
            GateStatus::Pass => "✅ PASS",
            GateStatus::Fail => "❌ FAIL",
            GateStatus::Waived => "⏭ WAIVED",
        };
        lines.push(format!("- {}: {} — {}", gate.gate, status, gate.message));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentforge_core::{DimensionScores, Scorecard};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_scorecard(agg: f64) -> Scorecard {
        Scorecard {
            run_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            agent_name: "test-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            aggregate_score: agg,
            pass_rate: agg,
            total_scenarios: 100,
            passed: (agg * 100.0) as u32,
            failed: 100 - (agg * 100.0) as u32,
            errors: 0,
            review_needed: 0,
            dimension_scores: DimensionScores {
                task_completion: agg,
                tool_selection: agg,
                argument_correctness: agg,
                schema_compliance: agg,
                instruction_adherence: agg,
                path_efficiency: agg,
            },
            failure_clusters: vec![],
            duration_seconds: 60,
            total_input_tokens: 1000,
            total_output_tokens: 500,
        }
    }

    fn make_trace(scenario_id: Uuid, status: TraceStatus) -> Trace {
        Trace {
            id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
            scenario_id,
            status,
            steps: vec![],
            final_output: None,
            scores: None,
            aggregate_score: None,
            failure_cluster: agentforge_core::FailureCluster::NoFailure,
            failure_reason: None,
            review_needed: false,
            llm_calls: 1,
            tool_invocations: 0,
            input_tokens: 50,
            output_tokens: 30,
            latency_ms: 500,
            retry_count: 0,
            seed: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn score_gate_passes_with_sufficient_delta() {
        let gk = Gatekeeper::new(GatekeeperConfig::default());
        let champion = make_scorecard(0.70);
        let challenger = make_scorecard(0.74);
        let result = gk
            .evaluate(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Some(&champion),
                &challenger,
                &[],
                &[],
                &[0.74, 0.73, 0.75],
            )
            .unwrap();
        assert!(result.approved);
    }

    #[test]
    fn score_gate_fails_below_delta() {
        let gk = Gatekeeper::new(GatekeeperConfig::default());
        let champion = make_scorecard(0.70);
        let challenger = make_scorecard(0.71); // only +1%, needs +3%
        let result = gk
            .evaluate(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Some(&champion),
                &challenger,
                &[],
                &[],
                &[0.71, 0.71, 0.71],
            )
            .unwrap();
        assert!(!result.approved);
        assert_eq!(result.gates[0].status, GateStatus::Fail);
    }

    #[test]
    fn waived_when_no_champion() {
        let gk = Gatekeeper::new(GatekeeperConfig::default());
        let challenger = make_scorecard(0.80);
        let result = gk
            .evaluate(
                Uuid::new_v4(),
                Uuid::new_v4(),
                None,
                &challenger,
                &[],
                &[],
                &[0.80, 0.79, 0.81],
            )
            .unwrap();
        // All gates should be waived or pass for first promotion
        assert!(result.approved, "First promotion should be approved");
        assert_eq!(result.gates[0].status, GateStatus::Waived);
        assert_eq!(result.gates[1].status, GateStatus::Waived);
    }

    #[test]
    fn regression_gate_detects_failures() {
        let gk = Gatekeeper::new(GatekeeperConfig {
            regression_gate_ratio: 0.99,
            score_gate_delta: 0.0, // disable score gate for this test
            stability_seeds: 1,
        });
        let champion = make_scorecard(0.80);

        // 100 champion-passing scenarios
        let scenario_ids: Vec<Uuid> = (0..100).map(|_| Uuid::new_v4()).collect();

        // Challenger only passes 90 of them
        let challenger_traces: Vec<Trace> = scenario_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                if i < 90 {
                    make_trace(id, TraceStatus::Pass)
                } else {
                    make_trace(id, TraceStatus::Fail)
                }
            })
            .collect();

        let challenger = make_scorecard(0.85);
        let result = gk
            .evaluate(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Some(&champion),
                &challenger,
                &scenario_ids,
                &challenger_traces,
                &[0.85],
            )
            .unwrap();
        assert!(!result.approved);
        assert_eq!(result.gates[1].status, GateStatus::Fail);
    }

    #[test]
    fn stability_gate_fails_high_variance() {
        let gk = Gatekeeper::new(GatekeeperConfig {
            stability_seeds: 3,
            score_gate_delta: 0.0,
            regression_gate_ratio: 0.0,
        });
        let challenger = make_scorecard(0.80);
        // Scores with high variance: 0.80 vs 0.70 = 0.10 delta (> 0.05 threshold)
        let result = gk
            .evaluate(
                Uuid::new_v4(),
                Uuid::new_v4(),
                None,
                &challenger,
                &[],
                &[],
                &[0.80, 0.74, 0.79],
            )
            .unwrap();
        assert!(!result.approved);
        assert_eq!(result.gates[2].status, GateStatus::Fail);
    }
}
