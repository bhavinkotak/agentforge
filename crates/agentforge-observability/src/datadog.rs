use agentforge_core::Trace;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::{ExporterError, TraceExporter};

/// Exports traces to Datadog APM via the Datadog Trace API.
///
/// Env vars:
/// - `DD_API_KEY` — Datadog API key
/// - `DD_SITE` — Datadog site (default: `datadoghq.com`)
/// - `DD_SERVICE` — service name tag (default: `agentforge`)
/// - `DD_ENV` — environment tag (default: `production`)
pub struct DatadogExporter {
    api_key: String,
    traces_url: String,
    service: String,
    env: String,
    client: Client,
}

impl DatadogExporter {
    pub fn new(api_key: String, site: String, service: String, env: String) -> Self {
        let traces_url = format!("https://trace.agent.{site}/api/v0.2/traces");
        Self {
            api_key,
            traces_url,
            service,
            env,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("DD_API_KEY").unwrap_or_default(),
            std::env::var("DD_SITE").unwrap_or_else(|_| "datadoghq.com".to_string()),
            std::env::var("DD_SERVICE").unwrap_or_else(|_| "agentforge".to_string()),
            std::env::var("DD_ENV").unwrap_or_else(|_| "production".to_string()),
        )
    }
}

#[async_trait]
impl TraceExporter for DatadogExporter {
    async fn export(&self, trace: &Trace) -> Result<(), ExporterError> {
        if self.api_key.is_empty() {
            return Err(ExporterError::Config("DD_API_KEY is not set".to_string()));
        }

        // Datadog APM expects trace_id and span_id as u64.
        let trace_id = trace.id.as_u128() as u64;
        let span_id = trace.run_id.as_u128() as u64;

        let span = json!({
            "trace_id": trace_id,
            "span_id": span_id,
            "name": "agentforge.eval",
            "resource": format!("scenario/{}", trace.scenario_id),
            "service": self.service,
            "type": "custom",
            "start": trace.created_at.timestamp_nanos_opt().unwrap_or(0),
            "duration": (trace.latency_ms * 1_000_000) as i64, // ns
            "meta": {
                "env": self.env,
                "run_id": trace.run_id.to_string(),
                "scenario_id": trace.scenario_id.to_string(),
                "status": trace.status.to_string(),
                "failure_cluster": trace.failure_cluster.to_string(),
                "aggregate_score": trace.aggregate_score.map(|s| s.to_string()).unwrap_or_default(),
            },
            "metrics": {
                "llm_calls": trace.llm_calls,
                "input_tokens": trace.input_tokens,
                "output_tokens": trace.output_tokens,
                "tool_invocations": trace.tool_invocations,
            }
        });

        // Datadog expects an array of arrays (list of traces, each trace = list of spans)
        let payload = json!([[span]]);

        self.client
            .put(&self.traces_url)
            .header("DD-API-KEY", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(ExporterError::Http)?;

        tracing::debug!(trace_id = %trace.id, "Exported trace to Datadog");
        Ok(())
    }
}
