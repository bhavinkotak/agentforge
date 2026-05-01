use agentforge_core::{
    AgentFile, AgentForgeError, EvalHints, ModelConfig, ModelProvider, Result, ToolDefinition,
};

/// Normalize AgentForge native YAML/JSON format into `AgentFile`.
pub fn normalize(value: &serde_json::Value) -> Result<AgentFile> {
    let name = string_field(value, "name")?;
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    let schema_version = value
        .get("agentforge_schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("1")
        .to_string();

    let model = parse_model(value)?;
    let system_prompt = string_field(value, "system_prompt")?;
    let tools = parse_tools(value)?;
    let output_schema = value.get("output_schema").cloned();
    let constraints = parse_constraints(value);
    let eval_hints = parse_eval_hints(value);

    Ok(AgentFile {
        agentforge_schema_version: schema_version,
        name,
        version,
        model,
        system_prompt,
        tools,
        output_schema,
        constraints,
        eval_hints,
        metadata: None,
    })
}

fn parse_model(value: &serde_json::Value) -> Result<ModelConfig> {
    let model_obj = value.get("model").ok_or_else(|| {
        AgentForgeError::ValidationError("Missing required field: model".to_string())
    })?;

    let provider_str = model_obj
        .get("provider")
        .and_then(|p| p.as_str())
        .unwrap_or("openai");

    let provider = parse_provider(provider_str);
    let model_id = model_obj
        .get("model_id")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            AgentForgeError::ValidationError("Missing required field: model.model_id".to_string())
        })?
        .to_string();

    Ok(ModelConfig {
        provider,
        model_id,
        temperature: model_obj.get("temperature").and_then(|t| t.as_f64()),
        max_tokens: model_obj
            .get("max_tokens")
            .and_then(|t| t.as_u64())
            .map(|v| v as u32),
        top_p: model_obj.get("top_p").and_then(|t| t.as_f64()),
    })
}

fn parse_tools(value: &serde_json::Value) -> Result<Vec<ToolDefinition>> {
    let tools_val = match value.get("tools") {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let arr = tools_val
        .as_array()
        .ok_or_else(|| AgentForgeError::ValidationError("tools must be an array".to_string()))?;

    arr.iter()
        .map(|t| {
            let name = t
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| AgentForgeError::ValidationError("Tool missing name".to_string()))?
                .to_string();
            let description = t
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let parameters = t
                .get("parameters")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

            Ok(ToolDefinition {
                name,
                description,
                parameters,
            })
        })
        .collect()
}

fn parse_constraints(value: &serde_json::Value) -> Vec<String> {
    value
        .get("constraints")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_eval_hints(value: &serde_json::Value) -> Option<EvalHints> {
    let hints = value.get("eval_hints")?;
    Some(EvalHints {
        domain: hints
            .get("domain")
            .and_then(|d| d.as_str())
            .map(String::from),
        typical_turns: hints
            .get("typical_turns")
            .and_then(|t| t.as_u64())
            .map(|v| v as u32),
        critical_tools: hints
            .get("critical_tools")
            .and_then(|ct| ct.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        pass_threshold: hints.get("pass_threshold").and_then(|t| t.as_f64()),
        scenario_count: hints
            .get("scenario_count")
            .and_then(|s| s.as_u64())
            .map(|v| v as u32),
    })
}

pub(crate) fn parse_provider(s: &str) -> ModelProvider {
    match s.to_lowercase().as_str() {
        "openai" => ModelProvider::Openai,
        "anthropic" => ModelProvider::Anthropic,
        "ollama" => ModelProvider::Ollama,
        "bedrock" => ModelProvider::Bedrock,
        "nvidia" | "nvidia_nim" => ModelProvider::NvidiaNim,
        _ => ModelProvider::Custom,
    }
}

fn string_field(value: &serde_json::Value, field: &str) -> Result<String> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AgentForgeError::ValidationError(format!("Missing required field: {field}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_valid_native_yaml() {
        let v = json!({
            "agentforge_schema_version": "1",
            "name": "test-agent",
            "version": "1.0.0",
            "model": {
                "provider": "openai",
                "model_id": "gpt-4o",
                "temperature": 0.2
            },
            "system_prompt": "You are a helpful assistant.",
            "tools": [],
            "constraints": ["Never hallucinate."]
        });
        let agent = normalize(&v).unwrap();
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.model.model_id, "gpt-4o");
        assert_eq!(agent.constraints.len(), 1);
    }

    #[test]
    fn rejects_missing_model() {
        let v = json!({
            "agentforge_schema_version": "1",
            "name": "test-agent",
            "system_prompt": "You are helpful."
        });
        assert!(normalize(&v).is_err());
    }

    #[test]
    fn parses_tools() {
        let v = json!({
            "agentforge_schema_version": "1",
            "name": "test-agent",
            "version": "1.0.0",
            "model": {"provider": "openai", "model_id": "gpt-4o"},
            "system_prompt": "You are helpful.",
            "tools": [
                {
                    "name": "get_order",
                    "description": "Get order details",
                    "parameters": {"type": "object", "properties": {"id": {"type": "string"}}}
                }
            ]
        });
        let agent = normalize(&v).unwrap();
        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.tools[0].name, "get_order");
    }
}
