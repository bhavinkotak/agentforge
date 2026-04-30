use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use agentforge_core::{AgentForgeError, ShadowRun, ShadowRunStatus};
use agentforge_db::{agent_repo::AgentRepo, shadow_repo::ShadowRepo};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct StartShadowRunRequest {
    pub champion_agent_id: Uuid,
    pub candidate_agent_id: Uuid,
    /// Percentage of traffic sent to candidate (1–100). Default: 10.
    pub traffic_percent: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct ShadowRunResponse {
    pub id: Uuid,
    pub champion_agent_id: Uuid,
    pub candidate_agent_id: Uuid,
    pub traffic_percent: u8,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ShadowRun> for ShadowRunResponse {
    fn from(r: ShadowRun) -> Self {
        Self {
            id: r.id,
            champion_agent_id: r.champion_agent_id,
            candidate_agent_id: r.candidate_agent_id,
            traffic_percent: r.traffic_percent,
            status: r.status.to_string(),
            created_at: r.created_at,
        }
    }
}

/// POST /shadow-runs — enqueue a new shadow run (returns 202 immediately).
pub async fn start_shadow_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartShadowRunRequest>,
) -> ApiResult<(StatusCode, Json<ShadowRunResponse>)> {
    let agent_repo = AgentRepo::new(state.db.clone());

    // Validate both agents exist.
    agent_repo
        .find_by_id(req.champion_agent_id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => ApiError::not_found(format!(
                "Champion agent {} not found",
                req.champion_agent_id
            )),
            other => ApiError::internal(other.to_string()),
        })?;

    agent_repo
        .find_by_id(req.candidate_agent_id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => ApiError::not_found(format!(
                "Candidate agent {} not found",
                req.candidate_agent_id
            )),
            other => ApiError::internal(other.to_string()),
        })?;

    let traffic_percent = req.traffic_percent.unwrap_or(10).clamp(1, 100);

    let run = ShadowRun {
        id: Uuid::new_v4(),
        champion_agent_id: req.champion_agent_id,
        candidate_agent_id: req.candidate_agent_id,
        traffic_percent,
        status: ShadowRunStatus::Pending,
        comparison: None,
        error_message: None,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
    };

    let shadow_repo = ShadowRepo::new(state.db.clone());
    let saved = shadow_repo
        .insert(&run)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    tracing::info!(
        shadow_run_id = %saved.id,
        champion = %saved.champion_agent_id,
        candidate = %saved.candidate_agent_id,
        traffic_percent = saved.traffic_percent,
        "Shadow run enqueued"
    );

    Ok((StatusCode::ACCEPTED, Json(saved.into())))
}

/// GET /shadow-runs/:id — get a shadow run by ID.
pub async fn get_shadow_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ShadowRunResponse>> {
    let shadow_repo = ShadowRepo::new(state.db.clone());
    let run = shadow_repo
        .find_by_id(id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => {
                ApiError::not_found(format!("Shadow run {id} not found"))
            }
            other => ApiError::internal(other.to_string()),
        })?;

    Ok(Json(run.into()))
}
