use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported export formats for fine-tuning datasets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// OpenAI fine-tuning JSONL format (chat messages).
    OpenAi,
    /// Anthropic fine-tuning JSONL format (prompt + completion pairs).
    Anthropic,
    /// HuggingFace datasets JSON-lines format.
    HuggingFace,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::OpenAi => write!(f, "openai"),
            ExportFormat::Anthropic => write!(f, "anthropic"),
            ExportFormat::HuggingFace => write!(f, "huggingface"),
        }
    }
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ExportFormat::OpenAi),
            "anthropic" => Ok(ExportFormat::Anthropic),
            "huggingface" | "hf" => Ok(ExportFormat::HuggingFace),
            other => Err(format!("Unknown export format: {other}")),
        }
    }
}

/// Status of a fine-tune export job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Pending,
    Running,
    Complete,
    Error,
}

impl std::fmt::Display for ExportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportStatus::Pending => write!(f, "pending"),
            ExportStatus::Running => write!(f, "running"),
            ExportStatus::Complete => write!(f, "complete"),
            ExportStatus::Error => write!(f, "error"),
        }
    }
}

/// Persisted record of a fine-tune export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneExport {
    pub id: Uuid,
    pub run_id: Uuid,
    pub format: ExportFormat,
    pub status: ExportStatus,
    /// Number of (prompt, completion) pairs exported.
    pub row_count: Option<u32>,
    /// Path to the exported file (local or S3 URI).
    pub file_path: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Minimum number of labeled traces required before export is allowed.
pub const MIN_TRACES_FOR_EXPORT: usize = 500;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_roundtrip() {
        for (s, expected) in [
            ("openai", ExportFormat::OpenAi),
            ("anthropic", ExportFormat::Anthropic),
            ("huggingface", ExportFormat::HuggingFace),
            ("hf", ExportFormat::HuggingFace),
        ] {
            let parsed: ExportFormat = s.parse().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(expected.to_string(), ExportFormat::from_str(s).unwrap().to_string());
        }
    }

    #[test]
    fn unknown_format_errors() {
        assert!("parquet".parse::<ExportFormat>().is_err());
    }
}
