use agentforge_core::{Trace, TraceStep};
use serde_json::{json, Value};

/// Convert passing traces to Anthropic fine-tuning JSONL format.
///
/// Format: `{"prompt": "\n\nHuman: ...\n\nAssistant:", "completion": "..."}`
pub fn convert(traces: &[&Trace]) -> Vec<Value> {
    traces.iter().filter_map(|t| trace_to_record(t)).collect()
}

fn trace_to_record(trace: &Trace) -> Option<Value> {
    let mut prompt_parts: Vec<String> = Vec::new();
    let mut completion: Option<String> = None;

    for step in &trace.steps {
        match step {
            TraceStep::LlmCall(call) => {
                if prompt_parts.is_empty() {
                    for msg in &call.messages {
                        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                        let content = msg
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or_default();
                        match role {
                            "system" => prompt_parts.push(format!("[SYSTEM]: {content}")),
                            "user" => prompt_parts.push(format!("\n\nHuman: {content}")),
                            "assistant" => prompt_parts.push(format!("\n\nAssistant: {content}")),
                            _ => {}
                        }
                    }
                }
                // Track last assistant completion
                if let Some(content) = call.response.get("content").and_then(|c| c.as_str()) {
                    completion = Some(content.to_string());
                }
            }
            TraceStep::FinalOutput(out) => {
                completion = Some(out.output.to_string());
            }
            _ => {}
        }
    }

    let completion_text = completion?;
    prompt_parts.push("\n\nAssistant:".to_string());

    Some(json!({
        "prompt": prompt_parts.join(""),
        "completion": completion_text,
    }))
}
