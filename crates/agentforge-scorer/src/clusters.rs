use agentforge_core::{DimensionScores, FailureCluster, Trace, TraceStep, TraceStatus};

/// Classify the primary failure reason for a trace into one of the known clusters.
pub fn classify_failure_cluster(
    trace: &Trace,
    scores: &DimensionScores,
    failure_reasons: &[String],
) -> FailureCluster {
    if trace.status == TraceStatus::Pass {
        return FailureCluster::NoFailure;
    }

    if trace.status == TraceStatus::Error {
        return FailureCluster::Unknown;
    }

    // Classify based on the lowest scoring dimension and failure reasons
    let failure_text = failure_reasons.join(" ").to_lowercase();

    // Schema violation: output schema compliance is very low
    if scores.schema_compliance < 0.3 {
        return FailureCluster::SchemaViolation;
    }

    // Wrong tool: tool selection accuracy is very low
    if scores.tool_selection < 0.3 {
        return FailureCluster::WrongTool;
    }

    // Hallucinated argument: argument correctness is very low
    if scores.argument_correctness < 0.3 {
        return FailureCluster::HallucinatedArgument;
    }

    // Looping: detect multiple repeated LLM calls with similar content
    if detect_loop(trace) {
        return FailureCluster::Looping;
    }

    // Premature stop: path efficiency is 0 (no tools called when needed)
    if scores.path_efficiency < 0.1 {
        return FailureCluster::PrematureStop;
    }

    // Constraint breach: instruction adherence is very low
    if scores.instruction_adherence < 0.3 {
        return FailureCluster::ConstraintBreach;
    }

    // Check failure reasons for keywords
    if failure_text.contains("wrong_tool") || failure_text.contains("missing required tools") {
        return FailureCluster::WrongTool;
    }
    if failure_text.contains("argument") || failure_text.contains("hallucinated") {
        return FailureCluster::HallucinatedArgument;
    }
    if failure_text.contains("schema") {
        return FailureCluster::SchemaViolation;
    }
    if failure_text.contains("constraint") || failure_text.contains("instruction adherence") {
        return FailureCluster::ConstraintBreach;
    }

    // Default: unknown
    FailureCluster::Unknown
}

/// Detect if the agent entered a loop (many repeated LLM calls with no tool calls between them).
fn detect_loop(trace: &Trace) -> bool {
    let llm_count = trace.steps.iter()
        .filter(|s| matches!(s, TraceStep::LlmCall(_)))
        .count();
    let tool_count = trace.steps.iter()
        .filter(|s| matches!(s, TraceStep::ToolCall(_)))
        .count();

    // Heuristic: >5 LLM calls with very few tool calls indicates looping
    llm_count > 5 && tool_count <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentforge_core::{FailureCluster, TraceStatus};

    fn make_scores(tool: f64, args: f64, schema: f64, adherence: f64, efficiency: f64) -> DimensionScores {
        DimensionScores {
            task_completion: 0.5,
            tool_selection: tool,
            argument_correctness: args,
            schema_compliance: schema,
            instruction_adherence: adherence,
            path_efficiency: efficiency,
        }
    }

    fn make_empty_trace(status: TraceStatus) -> Trace {
        Trace {
            id: uuid::Uuid::new_v4(),
            run_id: uuid::Uuid::new_v4(),
            scenario_id: uuid::Uuid::new_v4(),
            status,
            steps: vec![],
            final_output: None,
            scores: None,
            aggregate_score: None,
            failure_cluster: FailureCluster::Unknown,
            failure_reason: None,
            review_needed: false,
            llm_calls: 0,
            tool_invocations: 0,
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            retry_count: 0,
            seed: 0,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn pass_returns_no_failure() {
        let trace = make_empty_trace(TraceStatus::Pass);
        let scores = make_scores(1.0, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(
            classify_failure_cluster(&trace, &scores, &[]),
            FailureCluster::NoFailure
        );
    }

    #[test]
    fn error_returns_unknown() {
        let trace = make_empty_trace(TraceStatus::Error);
        let scores = make_scores(0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(
            classify_failure_cluster(&trace, &scores, &[]),
            FailureCluster::Unknown
        );
    }

    #[test]
    fn low_schema_compliance_is_schema_violation() {
        let mut trace = make_empty_trace(TraceStatus::Fail);
        let scores = make_scores(1.0, 1.0, 0.1, 1.0, 1.0);
        assert_eq!(
            classify_failure_cluster(&trace, &scores, &[]),
            FailureCluster::SchemaViolation
        );
    }

    #[test]
    fn low_tool_selection_is_wrong_tool() {
        let trace = make_empty_trace(TraceStatus::Fail);
        let scores = make_scores(0.1, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(
            classify_failure_cluster(&trace, &scores, &[]),
            FailureCluster::WrongTool
        );
    }

    #[test]
    fn low_args_is_hallucinated_argument() {
        let trace = make_empty_trace(TraceStatus::Fail);
        let scores = make_scores(1.0, 0.1, 1.0, 1.0, 1.0);
        assert_eq!(
            classify_failure_cluster(&trace, &scores, &[]),
            FailureCluster::HallucinatedArgument
        );
    }

    #[test]
    fn low_constraint_is_breach() {
        let trace = make_empty_trace(TraceStatus::Fail);
        let scores = make_scores(1.0, 1.0, 1.0, 0.1, 1.0);
        assert_eq!(
            classify_failure_cluster(&trace, &scores, &[]),
            FailureCluster::ConstraintBreach
        );
    }
}
