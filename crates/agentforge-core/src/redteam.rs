use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Category of a red-team probe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RedTeamCategory {
    /// Attempts to make the agent ignore its system prompt or constraints.
    Jailbreak,
    /// Injecting malicious instructions via user-controlled content.
    PromptInjection,
    /// Probing the agent to leak sensitive information (PII, API keys, internal data).
    DataLeakage,
    /// Confusing the agent about its role or persona.
    RoleConfusion,
    /// Bypassing declared behavioral constraints.
    ConstraintBypass,
}

impl std::fmt::Display for RedTeamCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedTeamCategory::Jailbreak => write!(f, "jailbreak"),
            RedTeamCategory::PromptInjection => write!(f, "prompt_injection"),
            RedTeamCategory::DataLeakage => write!(f, "data_leakage"),
            RedTeamCategory::RoleConfusion => write!(f, "role_confusion"),
            RedTeamCategory::ConstraintBypass => write!(f, "constraint_bypass"),
        }
    }
}

/// Result of a safety dimension score for a red-team trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyScore {
    /// Whether the agent resisted the attack (true = safe, false = violated).
    pub safe: bool,
    /// Category of the probe that was tested.
    pub category: RedTeamCategory,
    /// Confidence in this assessment (0.0–1.0).
    pub confidence: f64,
    /// Optional explanation from the LLM judge.
    pub rationale: Option<String>,
}

/// Summary of a red-team evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamSummary {
    pub run_id: Uuid,
    pub total_probes: u32,
    pub safe_count: u32,
    pub violated_count: u32,
    /// Safety rate: fraction of probes the agent resisted.
    pub safety_rate: f64,
    /// Per-category breakdown.
    pub by_category: Vec<RedTeamCategoryResult>,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamCategoryResult {
    pub category: RedTeamCategory,
    pub total: u32,
    pub safe: u32,
    pub safety_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_display() {
        assert_eq!(RedTeamCategory::Jailbreak.to_string(), "jailbreak");
        assert_eq!(
            RedTeamCategory::PromptInjection.to_string(),
            "prompt_injection"
        );
    }
}
