pub mod datadog;
pub mod langsmith;
pub mod otlp;

use agentforge_core::Trace;
use async_trait::async_trait;
use thiserror::Error;

/// Error type for trace exporters.
#[derive(Debug, Error)]
pub enum ExporterError {
    #[error("HTTP error exporting trace: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Export error: {0}")]
    Export(String),
}

/// Trait implemented by all observability backends.
#[async_trait]
pub trait TraceExporter: Send + Sync {
    /// Export a single trace to the backend.
    async fn export(&self, trace: &Trace) -> Result<(), ExporterError>;

    /// Export a batch of traces (default: loop over `export`).
    async fn export_batch(&self, traces: &[Trace]) -> Vec<Result<(), ExporterError>> {
        let mut results = Vec::with_capacity(traces.len());
        for trace in traces {
            results.push(self.export(trace).await);
        }
        results
    }
}

/// The active observability backend, selected at runtime by env vars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservabilityBackend {
    Otlp,
    LangSmith,
    Datadog,
    /// No-op exporter (observability disabled).
    Disabled,
}

impl ObservabilityBackend {
    /// Read from `AGENTFORGE_OBSERVABILITY_BACKEND` env var.
    pub fn from_env() -> Self {
        match std::env::var("AGENTFORGE_OBSERVABILITY_BACKEND")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "otlp" => ObservabilityBackend::Otlp,
            "langsmith" => ObservabilityBackend::LangSmith,
            "datadog" => ObservabilityBackend::Datadog,
            _ => ObservabilityBackend::Disabled,
        }
    }
}

/// A no-op exporter used when observability is disabled.
pub struct NoopExporter;

#[async_trait]
impl TraceExporter for NoopExporter {
    async fn export(&self, _trace: &Trace) -> Result<(), ExporterError> {
        Ok(())
    }
}

/// Build a `TraceExporter` based on environment configuration.
pub fn build_exporter() -> Box<dyn TraceExporter> {
    match ObservabilityBackend::from_env() {
        ObservabilityBackend::Otlp => {
            tracing::info!("Observability: OTLP exporter enabled");
            Box::new(otlp::OtlpExporter::from_env())
        }
        ObservabilityBackend::LangSmith => {
            tracing::info!("Observability: LangSmith exporter enabled");
            Box::new(langsmith::LangSmithExporter::from_env())
        }
        ObservabilityBackend::Datadog => {
            tracing::info!("Observability: Datadog exporter enabled");
            Box::new(datadog::DatadogExporter::from_env())
        }
        ObservabilityBackend::Disabled => {
            tracing::debug!("Observability: disabled (set AGENTFORGE_OBSERVABILITY_BACKEND to enable)");
            Box::new(NoopExporter)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_exporter_succeeds() {
        use agentforge_core::{FailureCluster, TraceStatus};
        use chrono::Utc;
        use uuid::Uuid;

        let trace = Trace {
            id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
            scenario_id: Uuid::new_v4(),
            status: TraceStatus::Pass,
            steps: vec![],
            final_output: None,
            scores: None,
            aggregate_score: Some(1.0),
            failure_cluster: FailureCluster::NoFailure,
            failure_reason: None,
            review_needed: false,
            llm_calls: 1,
            tool_invocations: 0,
            input_tokens: 100,
            output_tokens: 50,
            latency_ms: 200,
            retry_count: 0,
            seed: 42,
            created_at: Utc::now(),
        };

        let exporter = NoopExporter;
        assert!(exporter.export(&trace).await.is_ok());
    }
}
