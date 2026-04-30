use agentforge_core::{Trace, TraceStep};
use serde_json::{json, Value};

/// Convert passing traces to OpenAI fine-tuning JSONL format.
///
/// Format: `{"messages": [{"role": "system", ...}, {"role": "user", ...}, {"role": "assistant", ...}]}`
pub fn convert(traces: &[&Trace]) -> Vec<Value> {
    traces.iter().filter_map(|t| trace_to_record(t)).collect()
}

fn trace_to_record(trace: &Trace) -> Option<Value> {
    // Reconstruct conversation from trace steps
    let mut messages: Vec<Value> = Vec::new();

    // Collect LLM call messages from the first step
    let mut final_response: Option<Value> = None;

    for step in &trace.steps {
        match step {
            TraceStep::LlmCall(call) => {
                // Add input messages on first call only
                if messages.is_empty() {
                    for msg in &call.messages {
                        messages.push(msg.clone());
                    }
                }
                // Track the last assistant response
                if let Some(content) = call.response.get("content") {
                    final_response = Some(json!({
                        "role": "assistant",
                        "content": content
                    }));
                }
            }
            TraceStep::FinalOutput(out) => {
                final_response = Some(json!({
                    "role": "assistant",
                    "content": out.output.to_string()
                }));
            }
            _ => {}
        }
    }

    let assistant_msg = final_response?;
    messages.push(assistant_msg);

    if messages.is_empty() {
        return None;
    }

    Some(json!({ "messages": messages }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_traces_produce_no_records() {
        let records = convert(&[]);
        assert!(records.is_empty());
    }
}
