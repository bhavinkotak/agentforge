use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

mod error;
mod routes;
mod state;

pub use state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Agent endpoints
        .route("/agents", post(routes::agents::create_agent))
        .route("/agents", get(routes::agents::list_agents))
        .route("/agents/:id", get(routes::agents::get_agent))
        // Eval run endpoints
        .route("/runs", post(routes::runs::start_run))
        .route("/runs/:id", get(routes::runs::get_run))
        .route("/runs/:id/scorecard", get(routes::runs::get_scorecard))
        // Diff and promote
        .route("/diff", get(routes::diff::get_diff))
        .route("/promote/:run_id", post(routes::promote::promote_run))
        .with_state(state)
}
