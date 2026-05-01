use agentforge_core::Trace;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::{ExporterError, TraceExporter};

/// Exports traces to LangSmith's Runs API.
///
/// Env vars:
/// - `LANGSMITH_API_KEY` — LangSmith API key
/// - `LANGSMITH_PROJECT` — project name (default: `agentforge`)
/// - `LANGSMITH_API_URL` — API base URL (default: `https://api.smith.langchain.com`)
pub struct LangSmithExporter {
    api_key: String,
    project: String,
    api_url: String,
    client: Client,
}

impl LangSmithExporter {
    pub fn new(api_key: String, project: String, api_url: String) -> Self {
        Self {
            api_key,
            project,
            api_url,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("LANGSMITH_API_KEY").unwrap_or_default(),
            std::env::var("LANGSMITH_PROJECT").unwrap_or_else(|_| "agentforge".to_string()),
            std::env::var("LANGSMITH_API_URL")
                .unwrap_or_else(|_| "https://api.smith.langchain.com".to_string()),
        )
    }
}

#[async_trait]
impl TraceExporter for LangSmithExporter {
    async fn export(&self, trace: &Trace) -> Result<(), ExporterError> {
        if self.api_key.is_empty() {
            return Err(ExporterError::Config(
                "LANGSMITH_API_KEY is not set".to_string(),
            ));
        }

        let body = json!({
            "id": trace.id.to_string(),
            "name": format!("agentforge-trace-{}", &trace.id.to_string()[..8]),
            "run_type": "chain",
            "project_name": self.project,
            "inputs": {
                "scenario_id": trace.scenario_id.to_string(),
                "run_id": trace.run_id.to_string(),
            },
            "outputs": {
                "final_output": trace.final_output,
                "aggregate_score": trace.aggregate_score,
            },
            "extra": {
                "status": trace.status.to_string(),
                "latency_ms": trace.latency_ms,
                "llm_calls": trace.llm_calls,
                "input_tokens": trace.input_tokens,
                "output_tokens": trace.output_tokens,
                "failure_cluster": trace.failure_cluster.to_string(),
            },
        });

        self.client
            .post(format!("{}/api/v1/runs", self.api_url))
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(ExporterError::Http)?;

        tracing::debug!(trace_id = %trace.id, "Exported trace to LangSmith");
        Ok(())
    }
}
