use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentForgeError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("LLM error: {provider} - {message}")]
    LlmError { provider: String, message: String },

    #[error("LLM error: judge model and agent model must differ (both are {model})")]
    CircularBiasError { model: String },

    #[error("Scoring error: {0}")]
    ScoringError(String),

    #[error("Optimization error: {0}")]
    OptimizationError(String),

    #[error("Promotion failed: {reason}")]
    PromotionFailed { reason: String },

    #[error("Gatekeeper failed - score gate: current={current:.3} champion={champion:.3} required_delta={required:.3}")]
    ScoreGateFailed {
        current: f64,
        champion: f64,
        required: f64,
    },

    #[error("Gatekeeper failed - regression gate: pass_rate={pass_rate:.3} required={required:.3}")]
    RegressionGateFailed { pass_rate: f64, required: f64 },

    #[error("Gatekeeper failed - stability gate: only {seeds} seeds run, need {required}")]
    StabilityGateFailed { seeds: usize, required: usize },

    #[error("Not found: {resource} with id {id}")]
    NotFound { resource: &'static str, id: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("Rate limit exceeded for provider {provider}")]
    RateLimitExceeded { provider: String },
}

/// Convenience alias for `Result<T, AgentForgeError>`.
pub type Result<T> = std::result::Result<T, AgentForgeError>;

impl From<serde_json::Error> for AgentForgeError {
    fn from(e: serde_json::Error) -> Self {
        AgentForgeError::SerializationError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_score_gate() {
        let err = AgentForgeError::ScoreGateFailed {
            current: 0.82,
            champion: 0.85,
            required: 0.03,
        };
        let msg = err.to_string();
        assert!(msg.contains("score gate"));
        assert!(msg.contains("0.820"));
    }

    #[test]
    fn error_display_circular_bias() {
        let err = AgentForgeError::CircularBiasError {
            model: "gpt-4o".to_string(),
        };
        assert!(err.to_string().contains("gpt-4o"));
    }
}
