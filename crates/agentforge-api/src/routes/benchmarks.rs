use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use agentforge_core::{AgentForgeError, BenchmarkRun, BenchmarkSuite};
use agentforge_db::{agent_repo::AgentRepo, benchmark_repo::BenchmarkRepo};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct StartBenchmarkRequest {
    pub agent_id: Uuid,
    /// Benchmark suite: "gaia", "agentbench", or "webarena".
    pub suite: String,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkRunResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub suite: String,
    pub status: String,
    pub total_tasks: u32,
    pub correct: u32,
    pub accuracy: f64,
    pub percentile_rank: Option<f64>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<BenchmarkRun> for BenchmarkRunResponse {
    fn from(r: BenchmarkRun) -> Self {
        Self {
            id: r.id,
            agent_id: r.agent_id,
            suite: r.suite.to_string(),
            status: if r.completed_at.is_some() {
                "complete".to_string()
            } else {
                "pending".to_string()
            },
            total_tasks: r.total_tasks,
            correct: r.correct,
            accuracy: r.accuracy,
            percentile_rank: r.percentile_rank,
            started_at: r.started_at,
            completed_at: r.completed_at,
        }
    }
}

/// POST /benchmarks — enqueue a new benchmark run.
pub async fn start_benchmark(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartBenchmarkRequest>,
) -> ApiResult<(StatusCode, Json<BenchmarkRunResponse>)> {
    // Validate agent exists.
    let agent_repo = AgentRepo::new(state.db.clone());
    agent_repo
        .find_by_id(req.agent_id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => {
                ApiError::not_found(format!("Agent {} not found", req.agent_id))
            }
            other => ApiError::internal(other.to_string()),
        })?;

    let suite = parse_suite(&req.suite)
        .ok_or_else(|| ApiError::bad_request(format!("Unknown benchmark suite: {}", req.suite)))?;

    let run = BenchmarkRun {
        id: Uuid::new_v4(),
        agent_id: req.agent_id,
        suite,
        total_tasks: 0,
        correct: 0,
        accuracy: 0.0,
        percentile_rank: None,
        results: vec![],
        started_at: Utc::now(),
        completed_at: None,
    };

    let benchmark_repo = BenchmarkRepo::new(state.db.clone());
    let saved = benchmark_repo
        .insert_run(&run)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    tracing::info!(
        benchmark_run_id = %saved.id,
        agent_id = %saved.agent_id,
        suite = %saved.suite,
        "Benchmark run enqueued"
    );

    Ok((StatusCode::ACCEPTED, Json(saved.into())))
}

/// GET /benchmarks/:id — get a benchmark run by ID.
pub async fn get_benchmark(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BenchmarkRunResponse>> {
    let benchmark_repo = BenchmarkRepo::new(state.db.clone());
    let run = benchmark_repo
        .find_run_by_id(id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => {
                ApiError::not_found(format!("Benchmark run {id} not found"))
            }
            other => ApiError::internal(other.to_string()),
        })?;

    Ok(Json(run.into()))
}

fn parse_suite(s: &str) -> Option<BenchmarkSuite> {
    match s.to_lowercase().as_str() {
        "gaia" => Some(BenchmarkSuite::Gaia),
        "agentbench" => Some(BenchmarkSuite::AgentBench),
        "webarena" => Some(BenchmarkSuite::WebArena),
        _ => None,
    }
}
