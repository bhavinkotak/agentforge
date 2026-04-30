pub mod agent_repo;
pub mod eval_repo;
pub mod scenario_repo;
pub mod trace_repo;
pub mod pool;

pub use pool::*;
pub use agent_repo::AgentRepo;
pub use eval_repo::EvalRepo;
pub use scenario_repo::ScenarioRepo;
pub use trace_repo::TraceRepo;

use agentforge_core::AgentForgeError;

/// Convert sqlx errors to AgentForge errors.
pub(crate) fn db_err(e: sqlx::Error) -> AgentForgeError {
    AgentForgeError::DatabaseError(e.to_string())
}
