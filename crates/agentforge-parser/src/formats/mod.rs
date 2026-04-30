pub mod native;
pub mod openai;
pub mod anthropic;
pub mod crewai;

use agentforge_core::{AgentFile, AgentFileFormat, Result};

/// Convert a raw parsed value into the canonical `AgentFile` representation.
pub fn normalize(format: &AgentFileFormat, value: &serde_json::Value, raw: &str) -> Result<AgentFile> {
    match format {
        AgentFileFormat::NativeYaml => native::normalize(value),
        AgentFileFormat::OpenaiJson => openai::normalize(value),
        AgentFileFormat::AnthropicJson => anthropic::normalize(value),
        AgentFileFormat::CrewaiYaml => crewai::normalize(value),
        AgentFileFormat::LangchainYaml => {
            // LangChain: fall back to native normalization
            native::normalize(value)
        }
    }
}
