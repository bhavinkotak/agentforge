pub mod agent_repo;
pub mod benchmark_repo;
pub mod eval_repo;
pub mod finetune_repo;
pub mod pool;
pub mod scenario_repo;
pub mod shadow_repo;
pub mod trace_repo;

pub use agent_repo::AgentRepo;
pub use benchmark_repo::BenchmarkRepo;
pub use eval_repo::EvalRepo;
pub use finetune_repo::FineTuneRepo;
pub use pool::*;
pub use scenario_repo::ScenarioRepo;
pub use shadow_repo::ShadowRepo;
pub use trace_repo::TraceRepo;

use agentforge_core::AgentForgeError;

/// Convert sqlx errors to AgentForge errors.
pub(crate) fn db_err(e: sqlx::Error) -> AgentForgeError {
    AgentForgeError::DatabaseError(e.to_string())
}
