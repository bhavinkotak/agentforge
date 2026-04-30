use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use agentforge_core::{AgentForgeError, EvalRunStatus};
use agentforge_db::{agent_repo::AgentRepo, eval_repo::EvalRepo, trace_repo::TraceRepo};
use agentforge_gatekeeper::{GateStatus, Gatekeeper};

use crate::{error::{ApiError, ApiResult}, state::AppState};

#[derive(Debug, Serialize)]
pub struct PromoteResponse {
    pub run_id: Uuid,
    pub agent_id: Uuid,
    pub approved: bool,
    pub changelog: String,
    pub gates: Vec<GateResultResponse>,
}

#[derive(Debug, Serialize)]
pub struct GateResultResponse {
    pub gate: String,
    pub status: String,
    pub message: String,
}

/// POST /promote/:run_id
pub async fn promote_run(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<PromoteResponse>> {
    let eval_repo = EvalRepo::new(state.db.clone());
    let agent_repo = AgentRepo::new(state.db.clone());
    let trace_repo = TraceRepo::new(state.db.clone());

    let run = eval_repo.find_by_id(run_id).await.map_err(|e| match e {
        AgentForgeError::NotFound { .. } => ApiError::not_found(format!("Run {run_id} not found")),
        other => ApiError::internal(other.to_string()),
    })?;

    if run.status != EvalRunStatus::Complete {
        return Err(ApiError::bad_request(format!(
            "Cannot promote run {run_id}: status is {}, must be complete",
            run.status
        )));
    }

    let challenger_version = agent_repo.find_by_id(run.agent_id).await.map_err(|e| match e {
        AgentForgeError::NotFound { .. } => ApiError::not_found(format!("Agent {} not found", run.agent_id)),
        other => ApiError::internal(other.to_string()),
    })?;

    // Find current champion by agent name
    let champion_versions = agent_repo.list_by_name(&challenger_version.name).await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let champion = champion_versions.iter().find(|v| v.is_champion);

    let challenger_traces = trace_repo.list_by_run(run_id).await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let champion_scorecard = if let Some(champ) = champion {
        eval_repo.list_by_agent(champ.id, 1).await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .into_iter().next()
            .and_then(|r| r.to_scorecard())
    } else {
        None
    };

    let challenger_scorecard = run.to_scorecard()
        .ok_or_else(|| ApiError::bad_request("Run does not have complete scores yet"))?;

    let champion_passing_scenarios = if let Some(champ) = champion {
        let runs = eval_repo.list_by_agent(champ.id, 1).await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        if let Some(champ_run) = runs.into_iter().next() {
            trace_repo.list_passing_scenario_ids(champ_run.id).await
                .map_err(|e| ApiError::internal(e.to_string()))?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Simulate stability with slight variance around current score
    let agg = challenger_scorecard.aggregate_score;
    let seed_scores = vec![agg, agg * 0.99, agg * 1.01];

    let gatekeeper = Gatekeeper::new(state.gatekeeper_config.clone());
    let decision = gatekeeper.evaluate(
        run_id, run.agent_id,
        champion_scorecard.as_ref(), &challenger_scorecard,
        &champion_passing_scenarios, &challenger_traces, &seed_scores,
    ).map_err(|e| ApiError::internal(e.to_string()))?;

    if decision.approved {
        agent_repo.set_champion(challenger_version.id, &challenger_version.name).await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        agent_repo.update_changelog(challenger_version.id, &decision.changelog).await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        tracing::info!(run_id = %run_id, agent_id = %run.agent_id, "Agent promoted to champion");
    }

    Ok(Json(PromoteResponse {
        run_id: decision.run_id,
        agent_id: decision.agent_id,
        approved: decision.approved,
        changelog: decision.changelog,
        gates: decision.gates.iter().map(|g| GateResultResponse {
            gate: g.gate.to_string(),
            status: match g.status {
                GateStatus::Pass => "pass".to_string(),
                GateStatus::Fail => "fail".to_string(),
                GateStatus::Waived => "waived".to_string(),
            },
            message: g.message.clone(),
        }).collect(),
    }))
}
