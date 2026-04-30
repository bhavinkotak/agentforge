use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use agentforge_core::{AgentForgeError, ExportFormat, ExportStatus, FineTuneExport};
use agentforge_db::{eval_repo::EvalRepo, finetune_repo::FineTuneRepo};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct StartExportRequest {
    pub run_id: Uuid,
    /// Export format: "openai", "anthropic", or "huggingface". Default: "openai".
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FineTuneExportResponse {
    pub id: Uuid,
    pub run_id: Uuid,
    pub format: String,
    pub status: String,
    pub row_count: Option<u32>,
    pub file_path: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<FineTuneExport> for FineTuneExportResponse {
    fn from(e: FineTuneExport) -> Self {
        Self {
            id: e.id,
            run_id: e.run_id,
            format: e.format.to_string(),
            status: e.status.to_string(),
            row_count: e.row_count,
            file_path: e.file_path,
            created_at: e.created_at,
            completed_at: e.completed_at,
        }
    }
}

/// POST /exports/finetune — enqueue a fine-tune export job.
pub async fn start_export(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartExportRequest>,
) -> ApiResult<(StatusCode, Json<FineTuneExportResponse>)> {
    // Validate the eval run exists.
    let eval_repo = EvalRepo::new(state.db.clone());
    eval_repo
        .find_by_id(req.run_id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => {
                ApiError::not_found(format!("Eval run {} not found", req.run_id))
            }
            other => ApiError::internal(other.to_string()),
        })?;

    let format = req
        .format
        .as_deref()
        .unwrap_or("openai")
        .parse::<ExportFormat>()
        .map_err(|e| ApiError::bad_request(e))?;

    let export = FineTuneExport {
        id: Uuid::new_v4(),
        run_id: req.run_id,
        format,
        status: ExportStatus::Pending,
        row_count: None,
        file_path: None,
        error_message: None,
        created_at: Utc::now(),
        completed_at: None,
    };

    let finetune_repo = FineTuneRepo::new(state.db.clone());
    let saved = finetune_repo
        .insert(&export)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    tracing::info!(
        export_id = %saved.id,
        run_id = %saved.run_id,
        format = %saved.format,
        "Fine-tune export enqueued"
    );

    Ok((StatusCode::ACCEPTED, Json(saved.into())))
}

/// GET /exports/finetune/:id — get a fine-tune export by ID.
pub async fn get_export(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<FineTuneExportResponse>> {
    let finetune_repo = FineTuneRepo::new(state.db.clone());
    let export = finetune_repo
        .find_by_id(id)
        .await
        .map_err(|e| match e {
            AgentForgeError::NotFound { .. } => {
                ApiError::not_found(format!("Fine-tune export {id} not found"))
            }
            other => ApiError::internal(other.to_string()),
        })?;

    Ok(Json(export.into()))
}
