use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use agentforge_core::AgentVersion;
use agentforge_db::agent_repo::AgentRepo;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub v1: Uuid,
    pub v2: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DiffResponse {
    pub v1: AgentSummary,
    pub v2: AgentSummary,
    pub system_prompt_diff: Option<String>,
    pub tool_changes: ToolChanges,
    pub constraint_changes: ConstraintChanges,
}

#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub sha: String,
    pub is_champion: bool,
}

#[derive(Debug, Serialize)]
pub struct ToolChanges {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConstraintChanges {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl From<AgentVersion> for AgentSummary {
    fn from(v: AgentVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            version: v.version,
            sha: v.sha,
            is_champion: v.is_champion,
        }
    }
}

/// GET /diff?v1=<uuid>&v2=<uuid>
pub async fn get_diff(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiffQuery>,
) -> ApiResult<Json<DiffResponse>> {
    let repo = AgentRepo::new(state.db.clone());

    let v1 = repo.find_by_id(params.v1).await.map_err(|e| match e {
        agentforge_core::AgentForgeError::NotFound { .. } => {
            ApiError::not_found(format!("Agent version {} not found", params.v1))
        }
        other => ApiError::internal(other.to_string()),
    })?;

    let v2 = repo.find_by_id(params.v2).await.map_err(|e| match e {
        agentforge_core::AgentForgeError::NotFound { .. } => {
            ApiError::not_found(format!("Agent version {} not found", params.v2))
        }
        other => ApiError::internal(other.to_string()),
    })?;

    let diff = compute_diff(&v1, &v2);
    Ok(Json(diff))
}

fn compute_diff(v1: &AgentVersion, v2: &AgentVersion) -> DiffResponse {
    // System prompt diff (simple presence check — full diff would need external crate)
    let prompt1 = v1.file_content.system_prompt.as_str();
    let prompt2 = v2.file_content.system_prompt.as_str();
    let system_prompt_diff = if prompt1 != prompt2 {
        Some(format!(
            "- Version {} prompt differs from version {}",
            v1.version, v2.version
        ))
    } else {
        None
    };

    // Tool changes
    let tools1: Vec<String> = v1
        .file_content
        .tools
        .iter()
        .map(|t| t.name.clone())
        .collect();
    let tools2: Vec<String> = v2
        .file_content
        .tools
        .iter()
        .map(|t| t.name.clone())
        .collect();

    let added_tools: Vec<String> = tools2
        .iter()
        .filter(|t| !tools1.contains(t))
        .cloned()
        .collect();
    let removed_tools: Vec<String> = tools1
        .iter()
        .filter(|t| !tools2.contains(t))
        .cloned()
        .collect();

    // Constraint changes
    let constraints1: Vec<String> = v1.file_content.constraints.clone();
    let constraints2: Vec<String> = v2.file_content.constraints.clone();

    let added_constraints: Vec<String> = constraints2
        .iter()
        .filter(|c| !constraints1.contains(c))
        .cloned()
        .collect();
    let removed_constraints: Vec<String> = constraints1
        .iter()
        .filter(|c| !constraints2.contains(c))
        .cloned()
        .collect();

    DiffResponse {
        v1: v1.clone().into(),
        v2: v2.clone().into(),
        system_prompt_diff,
        tool_changes: ToolChanges {
            added: added_tools,
            removed: removed_tools,
            modified: vec![], // Would need deep comparison
        },
        constraint_changes: ConstraintChanges {
            added: added_constraints,
            removed: removed_constraints,
        },
    }
}
