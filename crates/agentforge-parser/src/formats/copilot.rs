use agentforge_core::{
    AgentFile, EvalHints, ModelConfig, ModelProvider, Result, ToolDefinition,
};
use std::collections::HashMap;

/// Normalize a GitHub Copilot `.agent.md` file into `AgentFile`.
///
/// Format: YAML frontmatter (`---`) followed by Markdown body.
/// Frontmatter fields:
///   - `name`         — agent display name (required)
///   - `description`  — short description (optional)
///   - `model`        — LLM model string, e.g. "GPT-4.1", "claude-sonnet-4-5" (optional)
///   - `tools`        — list of capability strings like "read", "github/*" (optional)
///
/// The Markdown body after the frontmatter becomes `system_prompt`.
pub fn normalize(frontmatter: &serde_json::Value, system_prompt_body: &str) -> Result<AgentFile> {
    let name = frontmatter
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("copilot-agent")
        .to_string();

    let description = frontmatter
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    // System prompt is the full Markdown body
    let system_prompt = system_prompt_body.trim().to_string();

    // Model: infer provider from model string
    let model = parse_model(frontmatter);

    // Tools: Copilot tools are capability reference strings (e.g. "github/*", "read"),
    // not structured tool definitions. We store them as minimal ToolDefinitions so they
    // appear in the agent representation and can be used in scenario generation.
    let tools = parse_copilot_tools(frontmatter);

    // Build metadata preserving Copilot-specific fields
    let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(desc) = &description {
        metadata.insert("description".to_string(), serde_json::Value::String(desc.clone()));
    }
    if let Some(arg_hint) = frontmatter.get("argument-hint").and_then(|v| v.as_str()) {
        metadata.insert("argument_hint".to_string(), serde_json::Value::String(arg_hint.to_string()));
    }
    if let Some(handoffs) = frontmatter.get("handoffs") {
        metadata.insert("handoffs".to_string(), handoffs.clone());
    }
    if let Some(mcp_servers) = frontmatter.get("mcp-servers") {
        metadata.insert("mcp_servers".to_string(), mcp_servers.clone());
    }

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
        metadata: if metadata.is_empty() { None } else { Some(metadata) },
    })
}

/// Parse the `model` frontmatter field into a `ModelConfig`.
/// Supports model strings like "GPT-4.1", "gpt-4o", "claude-sonnet-4-5", "o3", etc.
fn parse_model(frontmatter: &serde_json::Value) -> ModelConfig {
    let model_str = frontmatter
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o");

    let lower = model_str.to_lowercase();

    let (provider, model_id) = if lower.contains("claude") || lower.contains("anthropic") {
        (ModelProvider::Anthropic, model_str.to_string())
    } else if lower.contains("ollama") || lower.starts_with("ollama/") {
        let id = model_str.strip_prefix("ollama/").unwrap_or(model_str);
        (ModelProvider::Ollama, id.to_string())
    } else {
        // Default: OpenAI (covers gpt-*, o1, o3, GPT-4.1, etc.)
        (ModelProvider::Openai, model_str.to_string())
    };

    ModelConfig {
        provider,
        model_id,
        temperature: None,
        max_tokens: None,
        top_p: None,
    }
}

/// Parse Copilot tool capability strings into minimal `ToolDefinition` entries.
///
/// Copilot tools are references like `"read"`, `"github/*"`, `"context7/*"`, not full schemas.
/// We create lightweight ToolDefinitions so the rest of AgentForge can reason about them.
fn parse_copilot_tools(frontmatter: &serde_json::Value) -> Vec<ToolDefinition> {
    let tools_val = match frontmatter.get("tools") {
        Some(t) => t,
        None => return vec![],
    };

    let tool_refs: Vec<String> = match tools_val {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        serde_json::Value::String(s) => vec![s.clone()],
        _ => return vec![],
    };

    tool_refs
        .into_iter()
        .map(|capability| {
            // Derive a human-readable name: "github/*" → "github", "read/readFile" → "readFile"
            let display_name = capability
                .rsplit('/')
                .next()
                .map(|s| if s == "*" {
                    capability.split('/').next().unwrap_or(&capability).to_string()
                } else {
                    s.to_string()
                })
                .unwrap_or_else(|| capability.clone());

            ToolDefinition {
                name: display_name,
                description: format!("Copilot capability: {}", capability),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "x-copilot-capability": capability
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_basic_copilot_agent() {
        let frontmatter = serde_json::json!({
            "name": "GitHub Actions Expert",
            "description": "Specialist in secure CI/CD workflows",
            "model": "GPT-4.1",
            "tools": ["github/*", "search/codebase", "edit/editFiles"]
        });
        let body = "# GitHub Actions Expert\n\nYou help teams build secure workflows.";

        let agent = normalize(&frontmatter, body).unwrap();

        assert_eq!(agent.name, "GitHub Actions Expert");
        assert_eq!(agent.model.model_id, "GPT-4.1");
        assert_eq!(agent.model.provider, ModelProvider::Openai);
        assert!(agent.system_prompt.contains("GitHub Actions Expert"));
        assert_eq!(agent.tools.len(), 3);
        assert_eq!(agent.tools[0].name, "github");
        assert_eq!(agent.tools[1].name, "codebase");
        assert_eq!(agent.tools[2].name, "editFiles");
    }

    #[test]
    fn normalizes_claude_model() {
        let frontmatter = serde_json::json!({
            "name": "Claude Agent",
            "model": "claude-sonnet-4-5"
        });
        let agent = normalize(&frontmatter, "You are helpful.").unwrap();
        assert_eq!(agent.model.provider, ModelProvider::Anthropic);
        assert_eq!(agent.model.model_id, "claude-sonnet-4-5");
    }

    #[test]
    fn defaults_model_when_absent() {
        let frontmatter = serde_json::json!({ "name": "No Model Agent" });
        let agent = normalize(&frontmatter, "Do stuff.").unwrap();
        assert_eq!(agent.model.model_id, "gpt-4o");
        assert_eq!(agent.model.provider, ModelProvider::Openai);
    }

    #[test]
    fn stores_description_in_metadata() {
        let frontmatter = serde_json::json!({
            "name": "Test",
            "description": "A helpful test agent"
        });
        let agent = normalize(&frontmatter, "System prompt.").unwrap();
        let meta = agent.metadata.unwrap();
        assert_eq!(
            meta["description"],
            serde_json::Value::String("A helpful test agent".to_string())
        );
    }

    #[test]
    fn empty_tools_yields_no_tool_definitions() {
        let frontmatter = serde_json::json!({ "name": "No Tools" });
        let agent = normalize(&frontmatter, "Prompt.").unwrap();
        assert!(agent.tools.is_empty());
    }
}
