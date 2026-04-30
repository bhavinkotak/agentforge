use agentforge_core::{Trace, TraceStep};
use async_trait::async_trait;
use opentelemetry::{
    global,
    trace::{Span, SpanKind, TraceContextExt, Tracer},
    Context, KeyValue,
};

use crate::{ExporterError, TraceExporter};

/// Exports AgentForge traces as OpenTelemetry spans to any OTLP-compatible backend
/// (Jaeger, Grafana Tempo, Honeycomb, etc.).
///
/// Env vars:
/// - `AGENTFORGE_OTEL_ENDPOINT` — OTLP HTTP endpoint (default: `http://localhost:4318`)
/// - `AGENTFORGE_OTEL_SERVICE_NAME` — service name (default: `agentforge`)
pub struct OtlpExporter {
    endpoint: String,
    service_name: String,
}

impl OtlpExporter {
    pub fn new(endpoint: String, service_name: String) -> Self {
        Self { endpoint, service_name }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("AGENTFORGE_OTEL_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4318".to_string()),
            std::env::var("AGENTFORGE_OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "agentforge".to_string()),
        )
    }
}

#[async_trait]
impl TraceExporter for OtlpExporter {
    async fn export(&self, trace: &Trace) -> Result<(), ExporterError> {
        let tracer = global::tracer(self.service_name.clone());
        let cx = Context::new();

        let mut span = tracer
            .span_builder(format!("agentforge.trace.{}", trace.id))
            .with_kind(SpanKind::Internal)
            .start_with_context(&tracer, &cx);

        span.set_attribute(KeyValue::new("trace.id", trace.id.to_string()));
        span.set_attribute(KeyValue::new("run.id", trace.run_id.to_string()));
        span.set_attribute(KeyValue::new("scenario.id", trace.scenario_id.to_string()));
        span.set_attribute(KeyValue::new("status", trace.status.to_string()));
        span.set_attribute(KeyValue::new("latency_ms", trace.latency_ms as i64));
        span.set_attribute(KeyValue::new("llm_calls", trace.llm_calls as i64));
        span.set_attribute(KeyValue::new("input_tokens", trace.input_tokens as i64));
        span.set_attribute(KeyValue::new("output_tokens", trace.output_tokens as i64));

        if let Some(score) = trace.aggregate_score {
            span.set_attribute(KeyValue::new("aggregate_score", score.to_string()));
        }

        // Add step count attribute
        let step_count = trace.steps.len();
        span.set_attribute(KeyValue::new("step_count", step_count as i64));

        span.end();

        tracing::debug!(
            trace_id = %trace.id,
            endpoint = %self.endpoint,
            "Exported trace to OTLP"
        );

        Ok(())
    }
}
