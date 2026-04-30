use agentforge_core::{ExportFormat, Trace, TraceStatus, MIN_TRACES_FOR_EXPORT};
use thiserror::Error;

use crate::formats::{anthropic, huggingface, openai};

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Insufficient labeled traces: need {min}, have {have}")]
    InsufficientTraces { min: usize, have: usize },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Exports a set of labeled traces as fine-tuning records in the requested format.
pub struct FineTuneExporter;

impl FineTuneExporter {
    /// Export passing traces as fine-tuning records.
    ///
    /// Returns a `Vec` of JSON objects — one per (prompt, completion) pair.
    /// The caller serializes them to JSONL for writing to disk or uploading.
    ///
    /// # Errors
    /// Returns `ExportError::InsufficientTraces` if fewer than `MIN_TRACES_FOR_EXPORT`
    /// passing traces are provided.
    pub fn export(
        traces: &[Trace],
        format: &ExportFormat,
    ) -> Result<Vec<serde_json::Value>, ExportError> {
        let passing: Vec<&Trace> = traces
            .iter()
            .filter(|t| t.status == TraceStatus::Pass)
            .collect();

        if passing.len() < MIN_TRACES_FOR_EXPORT {
            return Err(ExportError::InsufficientTraces {
                min: MIN_TRACES_FOR_EXPORT,
                have: passing.len(),
            });
        }

        tracing::info!(
            format = %format,
            total_traces = traces.len(),
            passing_traces = passing.len(),
            "Exporting fine-tuning dataset"
        );

        let records: Vec<serde_json::Value> = match format {
            ExportFormat::OpenAi => openai::convert(&passing),
            ExportFormat::Anthropic => anthropic::convert(&passing),
            ExportFormat::HuggingFace => huggingface::convert(&passing),
        };

        Ok(records)
    }

    /// Serialize records to a JSONL string (one JSON object per line).
    pub fn to_jsonl(records: &[serde_json::Value]) -> Result<String, ExportError> {
        let mut lines = Vec::with_capacity(records.len());
        for record in records {
            lines.push(serde_json::to_string(record)?);
        }
        Ok(lines.join("\n"))
    }

    /// Check whether enough passing traces exist (without exporting).
    pub fn is_ready(traces: &[Trace]) -> bool {
        traces
            .iter()
            .filter(|t| t.status == TraceStatus::Pass)
            .count()
            >= MIN_TRACES_FOR_EXPORT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insufficient_traces_error() {
        let err = FineTuneExporter::export(&[], &ExportFormat::OpenAi).unwrap_err();
        assert!(matches!(err, ExportError::InsufficientTraces { .. }));
    }
}
