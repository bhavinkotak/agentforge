use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single test scenario generated for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub input: ScenarioInput,
    pub expected: ScenarioExpected,
    pub difficulty: DifficultyTier,
    pub domain: Option<String>,
    pub source: ScenarioSource,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// The input side of a scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioInput {
    /// User message or task description.
    pub user_message: String,
    /// Prior conversation turns, if any (multi-turn scenarios).
    pub conversation_history: Vec<ConversationTurn>,
    /// Any context injected into the conversation (e.g., user data).
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: ConversationRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    User,
    Assistant,
    System,
    Tool,
}

/// The expected outcomes for a scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioExpected {
    /// Tool calls that should be made (in order if sequence matters).
    pub tool_calls: Vec<ExpectedToolCall>,
    /// JSON Schema the final output must validate against.
    pub output_schema: Option<serde_json::Value>,
    /// Natural language description of what passing looks like (for LLM judge).
    pub pass_criteria: String,
    /// Minimum number of turns expected.
    pub min_turns: Option<u32>,
    /// Maximum number of turns allowed (looping detection).
    pub max_turns: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedToolCall {
    pub tool_name: String,
    /// Whether this tool MUST be called (required) or may be called (optional).
    pub required: bool,
    /// Partial argument schema to validate against (optional).
    pub argument_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DifficultyTier {
    Easy,
    Medium,
    Hard,
    Edge,
}

impl std::fmt::Display for DifficultyTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifficultyTier::Easy => write!(f, "easy"),
            DifficultyTier::Medium => write!(f, "medium"),
            DifficultyTier::Hard => write!(f, "hard"),
            DifficultyTier::Edge => write!(f, "edge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioSource {
    SchemaDerived,
    Adversarial,
    DomainSeeded,
    Manual,
}

impl std::fmt::Display for ScenarioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioSource::SchemaDerived => write!(f, "schema_derived"),
            ScenarioSource::Adversarial => write!(f, "adversarial"),
            ScenarioSource::DomainSeeded => write!(f, "domain_seeded"),
            ScenarioSource::Manual => write!(f, "manual"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_tier_display() {
        assert_eq!(DifficultyTier::Easy.to_string(), "easy");
        assert_eq!(DifficultyTier::Edge.to_string(), "edge");
    }

    #[test]
    fn scenario_source_display() {
        assert_eq!(ScenarioSource::SchemaDerived.to_string(), "schema_derived");
        assert_eq!(ScenarioSource::DomainSeeded.to_string(), "domain_seeded");
    }
}
