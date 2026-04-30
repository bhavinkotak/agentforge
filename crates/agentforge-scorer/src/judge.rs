use crate::scorer::ScorerConfig;
use agentforge_core::{
    AgentFile, DimensionScore, Result, Scenario, ScoringMethod, Trace, TraceStep,
};

/// Result of an LLM judge evaluation.
pub struct JudgeResult {
    pub task_completion: DimensionScore,
    pub instruction_adherence: DimensionScore,
}

/// Run the LLM judge for semantic scoring dimensions.
/// Enforces circular bias: judge_model must differ from the agent's model_id.
pub async fn run_llm_judge(
    trace: &Trace,
    scenario: &Scenario,
    agent: &AgentFile,
    config: &ScorerConfig,
) -> Result<JudgeResult> {
    // Circular bias guard: judge model must differ from the agent's model
    if config.judge_model == agent.model.model_id {
        tracing::warn!(
            judge = %config.judge_model,
            agent_model = %agent.model.model_id,
            "Judge model same as agent model — circular bias detected, using heuristic"
        );
        let tc = heuristic_task_completion(trace, scenario);
        let ia = heuristic_instruction_adherence(trace, agent);
        return Ok(JudgeResult {
            task_completion: tc,
            instruction_adherence: ia,
        });
    }

    // Build a minimal context for the judge
    let final_output = trace
        .final_output
        .as_ref()
        .map(|o| serde_json::to_string_pretty(o).unwrap_or_default())
        .unwrap_or_else(|| "<no output>".to_string());

    let tool_summary: String = trace
        .steps
        .iter()
        .filter_map(|s| {
            if let TraceStep::ToolCall(tc) = s {
                Some(format!(
                    "{}({})",
                    tc.tool_name,
                    serde_json::to_string(&tc.arguments).unwrap_or_default()
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    let prompt = format!(
        "You are an objective evaluator for an AI agent. Score the agent's performance on these two dimensions from 0.0 to 1.0:\n\n\
        Task: {task}\n\
        Pass Criteria: {criteria}\n\
        Agent Tool Calls: [{tools}]\n\
        Agent Final Output: {output}\n\n\
        Respond with JSON only:\n\
        {{\"task_completion\": <0.0-1.0>, \"task_confidence\": <0.0-1.0>, \"task_rationale\": \"...\",\n\
         \"instruction_adherence\": <0.0-1.0>, \"adherence_confidence\": <0.0-1.0>, \"adherence_rationale\": \"...\"}}",
        task = scenario.input.user_message,
        criteria = scenario.expected.pass_criteria,
        tools = tool_summary,
        output = final_output,
    );

    // Call the judge LLM via reqwest (OpenAI-compatible API)
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": config.judge_model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.0,
        "max_tokens": 300,
        "response_format": {"type": "json_object"}
    });

    let resp = client
        .post(format!("{}/chat/completions", config.judge_base_url))
        .bearer_auth(&config.judge_api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| agentforge_core::AgentForgeError::HttpError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(agentforge_core::AgentForgeError::LlmError {
            provider: "judge".to_string(),
            message: format!("HTTP {status}: {text}"),
        });
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| agentforge_core::AgentForgeError::ParseError(e.to_string()))?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");

    let parsed: serde_json::Value = serde_json::from_str(content).unwrap_or_default();

    let task_completion = DimensionScore {
        value: parsed["task_completion"]
            .as_f64()
            .unwrap_or(0.5)
            .clamp(0.0, 1.0),
        confidence: parsed["task_confidence"]
            .as_f64()
            .unwrap_or(0.7)
            .clamp(0.0, 1.0),
        method: ScoringMethod::LlmJudge,
        rationale: parsed["task_rationale"].as_str().map(String::from),
    };

    let instruction_adherence = DimensionScore {
        value: parsed["instruction_adherence"]
            .as_f64()
            .unwrap_or(0.5)
            .clamp(0.0, 1.0),
        confidence: parsed["adherence_confidence"]
            .as_f64()
            .unwrap_or(0.7)
            .clamp(0.0, 1.0),
        method: ScoringMethod::LlmJudge,
        rationale: parsed["adherence_rationale"].as_str().map(String::from),
    };

    Ok(JudgeResult {
        task_completion,
        instruction_adherence,
    })
}

/// Heuristic fallback for task completion when LLM judge is unavailable.
pub fn heuristic_task_completion(trace: &Trace, scenario: &Scenario) -> DimensionScore {
    // If agent has a final output and called some tools, assume partial success
    let has_output = trace.final_output.is_some();
    let called_required = {
        let required: Vec<&str> = scenario
            .expected
            .tool_calls
            .iter()
            .filter(|tc| tc.required)
            .map(|tc| tc.tool_name.as_str())
            .collect();
        let called: Vec<&str> = trace
            .steps
            .iter()
            .filter_map(|s| {
                if let TraceStep::ToolCall(tc) = s {
                    Some(tc.tool_name.as_str())
                } else {
                    None
                }
            })
            .collect();
        if required.is_empty() {
            true
        } else {
            required.iter().all(|r| called.contains(r))
        }
    };

    let value = match (has_output, called_required) {
        (true, true) => 0.8,
        (true, false) => 0.5,
        (false, true) => 0.4,
        (false, false) => 0.2,
    };

    DimensionScore {
        value,
        confidence: 0.4, // low confidence — heuristic only
        method: ScoringMethod::Heuristic,
        rationale: Some(format!(
            "Heuristic: has_output={has_output}, called_required={called_required}"
        )),
    }
}

/// Heuristic fallback for instruction adherence.
fn heuristic_instruction_adherence(trace: &Trace, agent: &AgentFile) -> DimensionScore {
    if agent.constraints.is_empty() {
        return DimensionScore {
            value: 1.0,
            confidence: 0.5,
            method: ScoringMethod::Heuristic,
            rationale: Some("No constraints defined".to_string()),
        };
    }

    // Collect all text from trace
    let all_text: String = trace
        .steps
        .iter()
        .filter_map(|s| {
            if let TraceStep::LlmCall(call) = s {
                call.response
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .map(String::from)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let value = if all_text.is_empty() { 0.5 } else { 0.7 };

    DimensionScore {
        value,
        confidence: 0.3,
        method: ScoringMethod::Heuristic,
        rationale: Some("Heuristic instruction adherence check".to_string()),
    }
}
