use agentforge_core::{
    AgentFile, AgentForgeError, EvalHints, ModelConfig, ModelProvider, Result, ToolDefinition,
};

/// Normalize CrewAI agent YAML into `AgentFile`.
/// CrewAI format: role, goal, backstory, tools[], llm
pub fn normalize(value: &serde_json::Value) -> Result<AgentFile> {
    // Handle both single agent and agents array
    let agent = if value.get("agents").is_some() {
        value
            .get("agents")
            .and_then(|a| a.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| AgentForgeError::ValidationError("CrewAI: 'agents' array is empty".to_string()))?
    } else {
        value
    };

    let role = agent
        .get("role")
        .and_then(|r| r.as_str())
        .ok_or_else(|| AgentForgeError::ValidationError("CrewAI: missing 'role'".to_string()))?;

    let goal = agent
        .get("goal")
        .and_then(|g| g.as_str())
        .unwrap_or("");

    let backstory = agent
        .get("backstory")
        .and_then(|b| b.as_str())
        .unwrap_or("");

    let name = agent
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(role)
        .to_string();

    // Construct system prompt from CrewAI fields
    let system_prompt = format!(
        "You are {role}.\n\nGoal: {goal}\n\nBackstory: {backstory}"
    );

    let model_id = agent
        .get("llm")
        .and_then(|l| l.as_str())
        .or_else(|| value.get("llm").and_then(|l| l.as_str()))
        .unwrap_or("gpt-4o")
        .to_string();

    let model = ModelConfig {
        provider: if model_id.contains("claude") {
            ModelProvider::Anthropic
        } else {
            ModelProvider::Openai
        },
        model_id,
        temperature: None,
        max_tokens: None,
        top_p: None,
    };

    // CrewAI tools are just tool names (strings), not full definitions
    let tools: Vec<ToolDefinition> = agent
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    if let Some(name) = t.as_str() {
                        Some(ToolDefinition {
                            name: name.to_string(),
                            description: format!("Tool: {name}"),
                            parameters: serde_json::json!({"type": "object", "properties": {}}),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_crewai_agent() {
        let v = json!({
            "role": "Support Specialist",
            "goal": "Help customers resolve issues",
            "backstory": "You are an expert support agent with 10 years of experience.",
            "llm": "gpt-4o",
            "tools": ["search_tool", "order_lookup_tool"]
        });
        let agent = normalize(&v).unwrap();
        assert!(agent.system_prompt.contains("Support Specialist"));
        assert!(agent.system_prompt.contains("Help customers"));
        assert_eq!(agent.tools.len(), 2);
    }

    #[test]
    fn normalizes_crewai_agents_array() {
        let v = json!({
            "agents": [
                {
                    "role": "Researcher",
                    "goal": "Research topics",
                    "backstory": "Expert researcher"
                }
            ]
        });
        let agent = normalize(&v).unwrap();
        assert!(agent.system_prompt.contains("Researcher"));
    }

    #[test]
    fn rejects_missing_role() {
        let v = json!({"goal": "Help", "backstory": "Expert"});
        assert!(normalize(&v).is_err());
    }
}
