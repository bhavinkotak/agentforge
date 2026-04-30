pub mod native;
pub mod openai;
pub mod anthropic;
pub mod crewai;
pub mod copilot;

use agentforge_core::{AgentFile, AgentFileFormat, Result};

/// Convert a raw parsed value into the canonical `AgentFile` representation.
///
/// For `CopilotAgentMd` the `raw` source is passed so we can extract the Markdown body
/// (everything after the closing `---` frontmatter delimiter) as the system prompt.
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
        AgentFileFormat::CopilotAgentMd => {
            // The system prompt is the Markdown body after the closing ---
            let body = extract_markdown_body(raw);
            copilot::normalize(value, body)
        }
    }
}

/// Extract the Markdown body that follows the YAML frontmatter block.
/// Given `---\nfrontmatter\n---\nbody`, returns the `body` part.
fn extract_markdown_body(raw: &str) -> &str {
    let trimmed = raw.trim();
    if !trimmed.starts_with("---") {
        return trimmed;
    }
    // Split on first two `---` occurrences: [empty, frontmatter, body]
    let mut parts = trimmed.splitn(3, "---");
    parts.next(); // before first ---
    parts.next(); // frontmatter
    parts.next().map(|s| s.trim()).unwrap_or("")
}
