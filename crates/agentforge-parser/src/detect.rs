use agentforge_core::{AgentFileFormat, AgentForgeError, Result};

/// Detect the format of an agent file from its raw content.
/// Detection order: JSON → Copilot .agent.md frontmatter → YAML field sniffing
pub fn detect_format(content: &str) -> Result<AgentFileFormat> {
    let trimmed = content.trim();

    // If it parses as JSON, classify it
    if trimmed.starts_with('{') {
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| AgentForgeError::ParseError(format!("Invalid JSON: {e}")))?;

        return Ok(classify_json_format(&value));
    }

    // Markdown frontmatter: --- ... ---
    if trimmed.starts_with("---") {
        let yaml_body = extract_frontmatter(trimmed)?;
        let value: serde_json::Value = serde_yaml::from_str(&yaml_body)
            .map_err(|e| AgentForgeError::ParseError(format!("Invalid frontmatter YAML: {e}")))?;

        // Copilot .agent.md: has `name` in frontmatter but NOT `agentforge_schema_version`,
        // and the body after the closing `---` contains non-trivial Markdown content.
        let has_name = value.get("name").is_some();
        let is_native = value.get("agentforge_schema_version").is_some();
        let has_markdown_body = has_markdown_body_after_frontmatter(trimmed);

        if has_name && !is_native && has_markdown_body {
            return Ok(AgentFileFormat::CopilotAgentMd);
        }

        return Ok(classify_yaml_format(&value));
    }

    // YAML
    if let Ok(value) = serde_yaml::from_str::<serde_json::Value>(trimmed) {
        return Ok(classify_yaml_format(&value));
    }

    Err(AgentForgeError::InvalidFormat(
        "Cannot detect format: not valid JSON, YAML, or Markdown frontmatter".to_string(),
    ))
}

/// Return true when the content after the closing `---` contains non-trivial Markdown.
fn has_markdown_body_after_frontmatter(content: &str) -> bool {
    let mut parts = content.splitn(3, "---");
    parts.next(); // empty prefix
    parts.next(); // frontmatter
    match parts.next() {
        Some(body) => !body.trim().is_empty(),
        None => false,
    }
}

fn classify_json_format(v: &serde_json::Value) -> AgentFileFormat {
    // OpenAI Assistants API: has "instructions" and "tools" array at root, no "system_prompt"
    if v.get("instructions").is_some()
        && v.get("tools").is_some()
        && v.get("system_prompt").is_none()
    {
        return AgentFileFormat::OpenaiJson;
    }

    // Anthropic: has "system" key at root with "tools" or "tool_choice"
    if (v.get("system").is_some() || v.get("system_prompt").is_some())
        && v.get("model").is_some()
        && v.get("agentforge_schema_version").is_none()
    {
        // Check for Anthropic model names
        if let Some(model) = v.get("model").and_then(|m| m.as_str()) {
            if model.contains("claude") {
                return AgentFileFormat::AnthropicJson;
            }
        }
        return AgentFileFormat::AnthropicJson;
    }

    // Native AgentForge JSON
    AgentFileFormat::NativeYaml
}

fn classify_yaml_format(v: &serde_json::Value) -> AgentFileFormat {
    // Native AgentForge YAML: has agentforge_schema_version
    if v.get("agentforge_schema_version").is_some() {
        return AgentFileFormat::NativeYaml;
    }

    // LangChain: has "_type" field set to "langchain" or similar
    if let Some(t) = v.get("_type").and_then(|t| t.as_str()) {
        if t.contains("langchain") || t.contains("lang_chain") {
            return AgentFileFormat::LangchainYaml;
        }
    }

    // CrewAI: has "agents" array at root or "role" + "goal" + "backstory"
    if v.get("agents").is_some()
        || (v.get("role").is_some() && v.get("goal").is_some() && v.get("backstory").is_some())
    {
        return AgentFileFormat::CrewaiYaml;
    }

    // Default to native YAML
    AgentFileFormat::NativeYaml
}

fn extract_frontmatter(content: &str) -> Result<String> {
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(AgentForgeError::ParseError(
            "Malformed Markdown frontmatter: missing closing ---".to_string(),
        ));
    }
    Ok(parts[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_native_yaml() {
        let content = r#"
agentforge_schema_version: "1"
name: test-agent
version: "1.0.0"
"#;
        assert_eq!(detect_format(content).unwrap(), AgentFileFormat::NativeYaml);
    }

    #[test]
    fn detects_openai_json() {
        let content = r#"{"instructions": "You are helpful.", "tools": [], "model": "gpt-4o"}"#;
        assert_eq!(detect_format(content).unwrap(), AgentFileFormat::OpenaiJson);
    }

    #[test]
    fn detects_anthropic_json() {
        let content =
            r#"{"system": "You are helpful.", "tools": [], "model": "claude-3-5-sonnet-20241022"}"#;
        assert_eq!(
            detect_format(content).unwrap(),
            AgentFileFormat::AnthropicJson
        );
    }

    #[test]
    fn detects_crewai_yaml() {
        let content = r#"
role: Support Agent
goal: Help customers
backstory: You are an expert support agent.
"#;
        assert_eq!(detect_format(content).unwrap(), AgentFileFormat::CrewaiYaml);
    }

    #[test]
    fn detects_markdown_frontmatter() {
        let content = r#"---
agentforge_schema_version: "1"
name: test-agent
version: "1.0.0"
---
# Documentation
"#;
        assert_eq!(detect_format(content).unwrap(), AgentFileFormat::NativeYaml);
    }

    #[test]
    fn detects_copilot_agent_md() {
        let content = r#"---
name: 'GitHub Actions Expert'
description: 'CI/CD workflow specialist'
model: GPT-4.1
tools: ['github/*', 'read']
---

# GitHub Actions Expert

You are a GitHub Actions specialist.
"#;
        assert_eq!(
            detect_format(content).unwrap(),
            AgentFileFormat::CopilotAgentMd
        );
    }

    #[test]
    fn copilot_md_without_body_falls_through_to_yaml() {
        // No Markdown body — treated as plain YAML (NativeYaml default)
        let content = "---\nname: 'Just Frontmatter'\n---\n";
        // Empty body so NOT CopilotAgentMd — falls through to NativeYaml
        assert_eq!(detect_format(content).unwrap(), AgentFileFormat::NativeYaml);
    }

    #[test]
    fn rejects_invalid_json() {
        let result = detect_format("{invalid json}");
        assert!(result.is_err());
    }
}
