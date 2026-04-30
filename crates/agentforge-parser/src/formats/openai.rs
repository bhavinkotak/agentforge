use agentforge_core::{
    AgentFile, AgentForgeError, EvalHints, ModelConfig, ModelProvider, Result, ToolDefinition,
};
use crate::formats::native::parse_provider;

/// Normalize OpenAI Assistants API JSON into `AgentFile`.
/// https://platform.openai.com/docs/api-reference/assistants
pub fn normalize(value: &serde_json::Value) -> Result<AgentFile> {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("openai-assistant")
        .to_string();

    let version = value
        .get("metadata")
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    // OpenAI uses "instructions" as the system prompt
    let system_prompt = value
        .get("instructions")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AgentForgeError::ValidationError("OpenAI: missing 'instructions' field".to_string()))?
        .to_string();

    let model_id = value
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AgentForgeError::ValidationError("OpenAI: missing 'model' field".to_string()))?
        .to_string();

    let temperature = value.get("temperature").and_then(|t| t.as_f64());
    let top_p = value.get("top_p").and_then(|t| t.as_f64());

    let model = ModelConfig {
        provider: ModelProvider::Openai,
        model_id,
        temperature,
        max_tokens: None,
        top_p,
    };

    // OpenAI tools are in a different format — they wrap function definitions
    let tools = parse_openai_tools(value)?;

    // OpenAI response format → output schema
    let output_schema = value
        .get("response_format")
        .and_then(|rf| rf.get("json_schema"))
        .cloned();

    Ok(AgentFile {
        agentforge_schema_version: "1".to_string(),
        name,
        version,
        model,
        system_prompt,
        tools,
        output_schema,
        constraints: vec![],
        eval_hints: Some(EvalHints::default()),
        metadata: None,
    })
}

fn parse_openai_tools(value: &serde_json::Value) -> Result<Vec<ToolDefinition>> {
    let tools_val = match value.get("tools") {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let arr = tools_val
        .as_array()
        .ok_or_else(|| AgentForgeError::ValidationError("tools must be an array".to_string()))?;

    arr.iter()
        .filter_map(|t| {
            // OpenAI tool format: {"type": "function", "function": {...}}
            if t.get("type").and_then(|ty| ty.as_str()) == Some("function") {
                let func = t.get("function")?;
                let name = func.get("name")?.as_str()?.to_string();
                let description = func
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let parameters = func
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
                Some(Ok(ToolDefinition { name, description, parameters }))
            } else {
                // Ignore non-function tools (code_interpreter, file_search, etc.)
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_openai_assistant() {
        let v = json!({
            "name": "support-bot",
            "instructions": "You are a helpful support agent.",
            "model": "gpt-4o",
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_order",
                        "description": "Get order details",
                        "parameters": {
                            "type": "object",
                            "properties": {"id": {"type": "string"}}
                        }
                    }
                }
            ]
        });
        let agent = normalize(&v).unwrap();
        assert_eq!(agent.name, "support-bot");
        assert_eq!(agent.system_prompt, "You are a helpful support agent.");
        assert_eq!(agent.model.model_id, "gpt-4o");
        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.tools[0].name, "get_order");
    }

    #[test]
    fn filters_non_function_tools() {
        let v = json!({
            "instructions": "You are helpful.",
            "model": "gpt-4o",
            "tools": [
                {"type": "code_interpreter"},
                {"type": "function", "function": {"name": "search", "description": "Search"}}
            ]
        });
        let agent = normalize(&v).unwrap();
        assert_eq!(agent.tools.len(), 1);
    }

    #[test]
    fn rejects_missing_instructions() {
        let v = json!({ "model": "gpt-4o" });
        assert!(normalize(&v).is_err());
    }
}
