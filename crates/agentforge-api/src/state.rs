use agentforge_db::PgPool;
use agentforge_gatekeeper::GatekeeperConfig;
use agentforge_runner::LlmClient;
use agentforge_scorer::ScorerConfig;
use std::sync::Arc;

/// Shared application state injected into all route handlers.
pub struct AppState {
    pub db: PgPool,
    pub llm_client: Arc<dyn LlmClient>,
    pub scorer_config: ScorerConfig,
    pub gatekeeper_config: GatekeeperConfig,
}
