use agentforge_core::{
    AgentFile, AgentForgeError, EvalHints, ModelConfig, ModelProvider, Result, ToolDefinition,
};

/// Normalize Anthropic system-prompt + tool block JSON into `AgentFile`.
pub fn normalize(value: &serde_json::Value) -> Result<AgentFile> {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("anthropic-agent")
        .to_string();

    // Anthropic uses "system" for the system prompt
    let system_prompt = value
        .get("system")
        .or_else(|| value.get("system_prompt"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AgentForgeError::ValidationError("Anthropic: missing 'system' field".to_string()))?
        .to_string();

    let model_id = value
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AgentForgeError::ValidationError("Anthropic: missing 'model' field".to_string()))?
        .to_string();

    let max_tokens = value
        .get("max_tokens")
        .and_then(|t| t.as_u64())
        .map(|v| v as u32);

    let temperature = value.get("temperature").and_then(|t| t.as_f64());

    let model = ModelConfig {
        provider: ModelProvider::Anthropic,
        model_id,
        temperature,
        max_tokens,
        top_p: None,
    };

    let tools = parse_anthropic_tools(value)?;

    Ok(AgentFile {
        agentforge_schema_version: "1".to_string(),
        name,
        version: "1.0.0".to_string(),
        model,
        system_prompt,
        tools,
        output_schema: None,
        constraints: vec![],
        eval_hints: Some(EvalHints::default()),
        metadata: None,
    })
}

fn parse_anthropic_tools(value: &serde_json::Value) -> Result<Vec<ToolDefinition>> {
    let tools_val = match value.get("tools") {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let arr = tools_val
        .as_array()
        .ok_or_else(|| AgentForgeError::ValidationError("tools must be an array".to_string()))?;

    arr.iter()
        .map(|t| {
            // Anthropic tool format: {"name": "...", "description": "...", "input_schema": {...}}
            let name = t
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| AgentForgeError::ValidationError("Anthropic tool missing name".to_string()))?
                .to_string();

            let description = t
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            // Anthropic uses "input_schema" where OpenAI uses "parameters"
            let parameters = t
                .get("input_schema")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

            Ok(ToolDefinition { name, description, parameters })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_anthropic_agent() {
        let v = json!({
            "model": "claude-3-5-sonnet-20241022",
            "system": "You are a helpful support agent.",
            "max_tokens": 1024,
            "tools": [
                {
                    "name": "get_order",
                    "description": "Get order details",
                    "input_schema": {
                        "type": "object",
                        "properties": {"order_id": {"type": "string"}},
                        "required": ["order_id"]
                    }
                }
            ]
        });
        let agent = normalize(&v).unwrap();
        assert_eq!(agent.model.provider, ModelProvider::Anthropic);
        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.tools[0].name, "get_order");
        assert_eq!(agent.model.max_tokens, Some(1024));
    }

    #[test]
    fn accepts_system_prompt_key() {
        let v = json!({
            "model": "claude-3-opus-20240229",
            "system_prompt": "You are helpful."
        });
        let agent = normalize(&v).unwrap();
        assert_eq!(agent.system_prompt, "You are helpful.");
    }
}
