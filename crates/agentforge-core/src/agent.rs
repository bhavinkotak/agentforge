use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The AgentForge native agent file schema (v1).
/// Also the normalized representation after parsing any supported format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentFile {
    pub agentforge_schema_version: String,
    pub name: String,
    pub version: String,
    pub model: ModelConfig,
    pub system_prompt: String,
    pub tools: Vec<ToolDefinition>,
    pub output_schema: Option<serde_json::Value>,
    pub constraints: Vec<String>,
    pub eval_hints: Option<EvalHints>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub model_id: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    Openai,
    Anthropic,
    Ollama,
    Bedrock,
    Custom,
}

impl std::fmt::Display for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelProvider::Openai => write!(f, "openai"),
            ModelProvider::Anthropic => write!(f, "anthropic"),
            ModelProvider::Ollama => write!(f, "ollama"),
            ModelProvider::Bedrock => write!(f, "bedrock"),
            ModelProvider::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalHints {
    pub domain: Option<String>,
    pub typical_turns: Option<u32>,
    pub critical_tools: Vec<String>,
    pub pass_threshold: Option<f64>,
    pub scenario_count: Option<u32>,
}

impl Default for EvalHints {
    fn default() -> Self {
        Self {
            domain: None,
            typical_turns: Some(3),
            critical_tools: vec![],
            pass_threshold: Some(0.85),
            scenario_count: Some(100),
        }
    }
}

/// Parsed and versioned agent file stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVersion {
    pub id: uuid::Uuid,
    pub name: String,
    pub version: String,
    pub sha: String,
    pub file_content: AgentFile,
    pub raw_content: String,
    pub format: AgentFileFormat,
    pub promoted: bool,
    pub is_champion: bool,
    pub changelog: Option<String>,
    pub parent_sha: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Supported agent file input formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentFileFormat {
    NativeYaml,
    OpenaiJson,
    AnthropicJson,
    LangchainYaml,
    CrewaiYaml,
    /// GitHub Copilot `.agent.md` format — YAML frontmatter + Markdown system prompt body.
    CopilotAgentMd,
}

impl std::fmt::Display for AgentFileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentFileFormat::NativeYaml => write!(f, "native_yaml"),
            AgentFileFormat::OpenaiJson => write!(f, "openai_json"),
            AgentFileFormat::AnthropicJson => write!(f, "anthropic_json"),
            AgentFileFormat::LangchainYaml => write!(f, "langchain_yaml"),
            AgentFileFormat::CrewaiYaml => write!(f, "crewai_yaml"),
            AgentFileFormat::CopilotAgentMd => write!(f, "copilot_agent_md"),
        }
    }
}

impl std::str::FromStr for AgentFileFormat {
    type Err = crate::AgentForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "native_yaml" => Ok(AgentFileFormat::NativeYaml),
            "openai_json" => Ok(AgentFileFormat::OpenaiJson),
            "anthropic_json" => Ok(AgentFileFormat::AnthropicJson),
            "langchain_yaml" => Ok(AgentFileFormat::LangchainYaml),
            "crewai_yaml" => Ok(AgentFileFormat::CrewaiYaml),
            "copilot_agent_md" => Ok(AgentFileFormat::CopilotAgentMd),
            _ => Err(crate::AgentForgeError::InvalidFormat(s.to_string())),
        }
    }
}

/// Lint error surfaced during agent file validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintError {
    pub field: String,
    pub message: String,
    pub severity: LintSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LintSeverity {
    Error,
    Warning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_provider_display() {
        assert_eq!(ModelProvider::Openai.to_string(), "openai");
        assert_eq!(ModelProvider::Anthropic.to_string(), "anthropic");
    }

    #[test]
    fn agent_file_format_roundtrip() {
        use std::str::FromStr;
        assert_eq!(
            AgentFileFormat::from_str("native_yaml").unwrap(),
            AgentFileFormat::NativeYaml
        );
        assert_eq!(AgentFileFormat::NativeYaml.to_string(), "native_yaml");
    }

    #[test]
    fn eval_hints_default() {
        let hints = EvalHints::default();
        assert_eq!(hints.pass_threshold, Some(0.85));
        assert_eq!(hints.scenario_count, Some(100));
    }
}
