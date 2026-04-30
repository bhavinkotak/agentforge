use agentforge_core::{
    AgentFile, DimensionScore, FailureCluster, Result, Scenario, ScenarioExpected,
    ScoringMethod, Trace, TraceStep, ToolCallStep,
};
use jsonschema::JSONSchema;

/// Result of all deterministic assertions for a single trace.
#[derive(Debug, Default)]
pub struct DeterministicResult {
    pub tool_selection: DimensionScore,
    pub argument_correctness: DimensionScore,
    pub schema_compliance: DimensionScore,
    pub instruction_adherence: DimensionScore,
    pub path_efficiency: DimensionScore,
    /// List of failure reasons to assist cluster classification
    pub failure_reasons: Vec<String>,
}

/// Run all deterministic assertions on a trace.
pub fn run_deterministic_checks(
    trace: &Trace,
    scenario: &Scenario,
    agent: &AgentFile,
) -> DeterministicResult {
    let mut result = DeterministicResult::default();

    // 1. Tool selection accuracy
    result.tool_selection = check_tool_selection(trace, scenario);
    if result.tool_selection.value < 1.0 {
        result.failure_reasons.push(format!(
            "Tool selection failed (score={:.2})", result.tool_selection.value
        ));
    }

    // 2. Argument correctness
    result.argument_correctness = check_argument_correctness(trace, scenario);
    if result.argument_correctness.value < 1.0 {
        result.failure_reasons.push(format!(
            "Argument correctness failed (score={:.2})", result.argument_correctness.value
        ));
    }

    // 3. Output schema compliance
    result.schema_compliance = check_schema_compliance(trace, scenario, agent);
    if result.schema_compliance.value < 1.0 {
        result.failure_reasons.push(format!(
            "Schema compliance failed (score={:.2})", result.schema_compliance.value
        ));
    }

    // 4. Instruction adherence (constraint keyword check)
    result.instruction_adherence = check_constraint_keywords(trace, agent);
    if result.instruction_adherence.value < 1.0 {
        result.failure_reasons.push(format!(
            "Instruction adherence failed (score={:.2})", result.instruction_adherence.value
        ));
    }

    // 5. Path efficiency
    result.path_efficiency = check_path_efficiency(trace, scenario);

    result
}

/// Check if the required tools were called in the right order.
fn check_tool_selection(trace: &Trace, scenario: &Scenario) -> DimensionScore {
    let required_tools: Vec<&str> = scenario.expected.tool_calls.iter()
        .filter(|tc| tc.required)
        .map(|tc| tc.tool_name.as_str())
        .collect();

    if required_tools.is_empty() {
        // No required tools — trivially pass
        return DimensionScore {
            value: 1.0,
            confidence: 1.0,
            method: ScoringMethod::Deterministic,
            rationale: Some("No required tools specified".to_string()),
        };
    }

    let called_tools: Vec<&str> = trace.steps.iter()
        .filter_map(|s| {
            if let TraceStep::ToolCall(tc) = s {
                Some(tc.tool_name.as_str())
            } else {
                None
            }
        })
        .collect();

    let matched = required_tools.iter()
        .filter(|&&rt| called_tools.contains(&rt))
        .count();

    let score = matched as f64 / required_tools.len() as f64;
    let rationale = if score < 1.0 {
        let missing: Vec<&str> = required_tools.iter()
            .filter(|&&rt| !called_tools.contains(&rt))
            .copied()
            .collect();
        Some(format!("Missing required tools: {}", missing.join(", ")))
    } else {
        Some("All required tools called".to_string())
    };

    DimensionScore {
        value: score,
        confidence: 1.0,
        method: ScoringMethod::Deterministic,
        rationale,
    }
}

/// Check that tool arguments match the expected schemas.
fn check_argument_correctness(trace: &Trace, scenario: &Scenario) -> DimensionScore {
    let tool_calls: Vec<&ToolCallStep> = trace.steps.iter()
        .filter_map(|s| if let TraceStep::ToolCall(tc) = s { Some(tc) } else { None })
        .collect();

    if tool_calls.is_empty() {
        // If no tools were expected, this is a pass; otherwise score 0
        let has_required = scenario.expected.tool_calls.iter().any(|tc| tc.required);
        return DimensionScore {
            value: if has_required { 0.0 } else { 1.0 },
            confidence: 1.0,
            method: ScoringMethod::Deterministic,
            rationale: if has_required {
                Some("Required tools were not called".to_string())
            } else {
                Some("No tools called and none required".to_string())
            },
        };
    }

    let mut total_checks = 0usize;
    let mut passed_checks = 0usize;
    let mut failure_details = Vec::new();

    for tc_step in &tool_calls {
        // Find the expected tool definition for this tool name
        let expected = scenario.expected.tool_calls.iter()
            .find(|etc| etc.tool_name == tc_step.tool_name);

        if let Some(exp) = expected {
            if let Some(arg_schema) = &exp.argument_schema {
                total_checks += 1;
                match validate_against_schema(&tc_step.arguments, arg_schema) {
                    Ok(true) => { passed_checks += 1; }
                    Ok(false) => {
                        failure_details.push(format!(
                            "Tool '{}' argument schema validation failed", tc_step.tool_name
                        ));
                    }
                    Err(e) => {
                        failure_details.push(format!(
                            "Tool '{}' schema error: {}", tc_step.tool_name, e
                        ));
                    }
                }
            } else {
                // No argument schema to check against — trivially pass
                total_checks += 1;
                passed_checks += 1;
            }
        }
    }

    let score = if total_checks == 0 {
        1.0
    } else {
        passed_checks as f64 / total_checks as f64
    };

    DimensionScore {
        value: score,
        confidence: 1.0,
        method: ScoringMethod::Deterministic,
        rationale: if failure_details.is_empty() {
            Some("All tool arguments valid".to_string())
        } else {
            Some(failure_details.join("; "))
        },
    }
}

/// Validate a value against a JSON Schema.
fn validate_against_schema(value: &serde_json::Value, schema: &serde_json::Value) -> Result<bool> {
    match JSONSchema::compile(schema) {
        Ok(compiled) => Ok(compiled.is_valid(value)),
        Err(e) => {
            // Invalid schema — can't validate, return true to avoid false negatives
            tracing::warn!(error = %e, "Invalid JSON schema in scenario");
            Ok(true)
        }
    }
}

/// Check that the final output conforms to the declared output schema.
fn check_schema_compliance(trace: &Trace, scenario: &Scenario, agent: &AgentFile) -> DimensionScore {
    // Use scenario's expected output schema if present, else agent's output schema
    let schema = scenario.expected.output_schema.as_ref()
        .or(agent.output_schema.as_ref());

    let schema = match schema {
        Some(s) => s,
        None => return DimensionScore {
            value: 1.0,
            confidence: 0.5,
            method: ScoringMethod::Deterministic,
            rationale: Some("No output schema defined — skipping compliance check".to_string()),
        },
    };

    let output = match &trace.final_output {
        Some(o) => o,
        None => return DimensionScore {
            value: 0.0,
            confidence: 1.0,
            method: ScoringMethod::Deterministic,
            rationale: Some("No final output captured".to_string()),
        },
    };

    match validate_against_schema(output, schema) {
        Ok(valid) => DimensionScore {
            value: if valid { 1.0 } else { 0.0 },
            confidence: 1.0,
            method: ScoringMethod::Deterministic,
            rationale: Some(if valid {
                "Output matches schema".to_string()
            } else {
                "Output does not match schema".to_string()
            }),
        },
        Err(e) => DimensionScore {
            value: 0.5,
            confidence: 0.3,
            method: ScoringMethod::Deterministic,
            rationale: Some(format!("Schema validation error: {e}")),
        },
    }
}

/// Check that the agent's output does not violate constraint keywords.
fn check_constraint_keywords(trace: &Trace, agent: &AgentFile) -> DimensionScore {
    if agent.constraints.is_empty() {
        return DimensionScore {
            value: 1.0,
            confidence: 0.5,
            method: ScoringMethod::Deterministic,
            rationale: Some("No constraints defined".to_string()),
        };
    }

    // Get all text content from the trace steps
    let all_text = collect_assistant_text(trace);
    if all_text.is_empty() {
        return DimensionScore {
            value: 1.0,
            confidence: 0.5,
            method: ScoringMethod::Deterministic,
            rationale: Some("No assistant text to check".to_string()),
        };
    }

    let all_text_lower = all_text.to_lowercase();

    // Check constraints that have "never" or "do not" patterns — these are keyword-checkable
    let mut violations = Vec::new();
    for constraint in &agent.constraints {
        let c_lower = constraint.to_lowercase();
        // Extract the forbidden content (after "never" or "do not")
        if let Some(after_never) = c_lower.strip_prefix("never ") {
            // The forbidden content is roughly the rest of the constraint
            // We check for exact keyword matches in the output
            let forbidden_words: Vec<&str> = after_never.split_whitespace().take(3).collect();
            for word in forbidden_words {
                if word.len() > 4 && all_text_lower.contains(word) {
                    // Heuristic: if the agent output contains words from the "never X" constraint,
                    // flag it. This is imperfect — LLM judge will do semantic checking.
                    violations.push(format!("Potential constraint breach: '{}'", constraint));
                    break;
                }
            }
        }
    }

    let score = if violations.is_empty() { 1.0 } else { 0.0 };
    DimensionScore {
        value: score,
        confidence: if violations.is_empty() { 0.7 } else { 0.9 },
        method: ScoringMethod::Deterministic,
        rationale: if violations.is_empty() {
            Some("No obvious constraint violations detected".to_string())
        } else {
            Some(violations.join("; "))
        },
    }
}

/// Check path efficiency: compare actual tool calls to the minimum expected.
fn check_path_efficiency(trace: &Trace, scenario: &Scenario) -> DimensionScore {
    let expected_min = scenario.expected.tool_calls.iter()
        .filter(|tc| tc.required)
        .count();

    let actual_calls = trace.steps.iter()
        .filter(|s| matches!(s, TraceStep::ToolCall(_)))
        .count();

    if expected_min == 0 {
        // No expected tool calls — efficiency is 1.0 if agent didn't loop
        let llm_call_count = trace.steps.iter()
            .filter(|s| matches!(s, TraceStep::LlmCall(_)))
            .count();
        let score = if llm_call_count <= 3 { 1.0 } else { 0.5 };
        return DimensionScore {
            value: score,
            confidence: 0.8,
            method: ScoringMethod::Deterministic,
            rationale: Some(format!("{} LLM calls for a no-tool scenario", llm_call_count)),
        };
    }

    let score = if actual_calls == 0 {
        0.0
    } else if actual_calls <= expected_min {
        1.0
    } else {
        // Penalize extra calls: score = expected/actual (capped at 1.0)
        (expected_min as f64 / actual_calls as f64).min(1.0)
    };

    DimensionScore {
        value: score,
        confidence: 0.9,
        method: ScoringMethod::Deterministic,
        rationale: Some(format!(
            "Expected ~{} tool calls, actual {} calls",
            expected_min, actual_calls
        )),
    }
}

fn collect_assistant_text(trace: &Trace) -> String {
    let mut texts = Vec::new();
    for step in &trace.steps {
        if let TraceStep::FinalOutput(fo) = step {
            if let Some(resp) = fo.output.get("response").and_then(|r| r.as_str()) {
                texts.push(resp.to_string());
            }
        }
    }
    texts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentforge_core::{
        DifficultyTier, EvalHints, ExpectedToolCall, FailureCluster,
        FinalOutputStep, LlmCallStep, ModelConfig, ModelProvider, ScenarioExpected,
        ScenarioInput, ScenarioSource, ToolCallStep, TraceStatus,
    };
    use chrono::Utc;
    use uuid::Uuid;

    fn make_scenario(tool_name: &str, required: bool) -> Scenario {
        Scenario {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            input: ScenarioInput {
                user_message: "test".to_string(),
                conversation_history: vec![],
                context: None,
            },
            expected: ScenarioExpected {
                tool_calls: vec![ExpectedToolCall {
                    tool_name: tool_name.to_string(),
                    required,
                    argument_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {"id": {"type": "string"}},
                        "required": ["id"]
                    })),
                }],
                output_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"response": {"type": "string"}},
                    "required": ["response"]
                })),
                pass_criteria: "Agent should call the tool".to_string(),
                min_turns: Some(1),
                max_turns: Some(5),
            },
            difficulty: DifficultyTier::Easy,
            domain: None,
            source: ScenarioSource::SchemaDerived,
            tags: vec![],
            created_at: Utc::now(),
        }
    }

    fn make_trace_with_tool_call(tool_name: &str, args: serde_json::Value) -> Trace {
        let run_id = Uuid::new_v4();
        let scenario_id = Uuid::new_v4();
        Trace {
            id: Uuid::new_v4(),
            run_id,
            scenario_id,
            status: TraceStatus::Pass,
            steps: vec![
                TraceStep::ToolCall(ToolCallStep {
                    index: 0,
                    tool_name: tool_name.to_string(),
                    call_id: "call_1".to_string(),
                    arguments: args,
                    timestamp: Utc::now(),
                }),
                TraceStep::FinalOutput(FinalOutputStep {
                    index: 1,
                    output: serde_json::json!({"response": "Order status is 'shipped'."}),
                    timestamp: Utc::now(),
                }),
            ],
            final_output: Some(serde_json::json!({"response": "Order status is 'shipped'."})),
            scores: None,
            aggregate_score: None,
            failure_cluster: FailureCluster::NoFailure,
            failure_reason: None,
            review_needed: false,
            llm_calls: 1,
            tool_invocations: 1,
            input_tokens: 50,
            output_tokens: 20,
            latency_ms: 500,
            retry_count: 0,
            seed: 0,
            created_at: Utc::now(),
        }
    }

    fn make_simple_agent() -> AgentFile {
        AgentFile {
            agentforge_schema_version: "1".to_string(),
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            model: ModelConfig {
                provider: ModelProvider::Openai,
                model_id: "gpt-4o".to_string(),
                temperature: None,
                max_tokens: None,
                top_p: None,
            },
            system_prompt: "You are helpful.".to_string(),
            tools: vec![],
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {"response": {"type": "string"}},
                "required": ["response"]
            })),
            constraints: vec!["Never share passwords.".to_string()],
            eval_hints: None,
            metadata: None,
        }
    }

    #[test]
    fn tool_selection_passes_when_required_tool_called() {
        let scenario = make_scenario("get_order", true);
        let trace = make_trace_with_tool_call("get_order", serde_json::json!({"id": "ORD-123"}));
        let score = check_tool_selection(&trace, &scenario);
        assert_eq!(score.value, 1.0);
    }

    #[test]
    fn tool_selection_fails_when_required_tool_missing() {
        let scenario = make_scenario("get_order", true);
        // Trace calls the wrong tool
        let trace = make_trace_with_tool_call("wrong_tool", serde_json::json!({}));
        let score = check_tool_selection(&trace, &scenario);
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn schema_compliance_passes_for_valid_output() {
        let scenario = make_scenario("get_order", false);
        let trace = make_trace_with_tool_call("get_order", serde_json::json!({"id": "ORD-123"}));
        let agent = make_simple_agent();
        let score = check_schema_compliance(&trace, &scenario, &agent);
        assert_eq!(score.value, 1.0);
    }

    #[test]
    fn schema_compliance_fails_for_missing_required_field() {
        let scenario = make_scenario("get_order", false);
        let mut trace = make_trace_with_tool_call("get_order", serde_json::json!({"id": "ORD-123"}));
        // Override final_output with invalid data
        trace.final_output = Some(serde_json::json!({"action_taken": "resolved"})); // missing "response"
        let agent = make_simple_agent();
        let score = check_schema_compliance(&trace, &scenario, &agent);
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn constraint_check_passes_when_no_violations() {
        let trace = make_trace_with_tool_call("get_order", serde_json::json!({}));
        let agent = make_simple_agent();
        let score = check_constraint_keywords(&trace, &agent);
        // "Never share passwords" — trace doesn't mention passwords
        assert_eq!(score.value, 1.0);
    }

    #[test]
    fn argument_correctness_passes_for_valid_args() {
        let scenario = make_scenario("get_order", true);
        let trace = make_trace_with_tool_call("get_order", serde_json::json!({"id": "ORD-123"}));
        let score = check_argument_correctness(&trace, &scenario);
        assert_eq!(score.value, 1.0);
    }
}
