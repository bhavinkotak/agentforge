use agentforge_core::{Trace, TraceStep};
use serde_json::{json, Value};

/// Convert passing traces to HuggingFace datasets JSON-lines format.
///
/// Format: `{"input": "...", "output": "...", "trace_id": "..."}`
pub fn convert(traces: &[&Trace]) -> Vec<Value> {
    traces.iter().filter_map(|t| trace_to_record(t)).collect()
}

fn trace_to_record(trace: &Trace) -> Option<Value> {
    let mut input_text: Option<String> = None;
    let mut output_text: Option<String> = None;

    for step in &trace.steps {
        match step {
            TraceStep::LlmCall(call) => {
                if input_text.is_none() {
                    // Extract user message from the first LLM call
                    let user_msg = call.messages.iter().find(|m| {
                        m.get("role").and_then(|r| r.as_str()) == Some("user")
                    });
                    if let Some(msg) = user_msg {
                        input_text = msg
                            .get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());
                    }
                }
                if let Some(content) = call.response.get("content").and_then(|c| c.as_str()) {
                    output_text = Some(content.to_string());
                }
            }
            TraceStep::FinalOutput(out) => {
                output_text = Some(out.output.to_string());
            }
            _ => {}
        }
    }

    let input = input_text?;
    let output = output_text?;

    Some(json!({
        "trace_id": trace.id.to_string(),
        "input": input,
        "output": output,
        "aggregate_score": trace.aggregate_score,
        "status": trace.status.to_string(),
    }))
}
