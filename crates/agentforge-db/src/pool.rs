use agentforge_core::AgentForgeError;
use sqlx::postgres::PgPoolOptions;

pub use sqlx::PgPool;

/// Create a PostgreSQL connection pool.
pub async fn create_pool(database_url: &str) -> Result<PgPool, AgentForgeError> {
    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await
        .map_err(|e| AgentForgeError::DatabaseError(format!("Failed to connect: {e}")))
}

/// Run all pending migrations from the migrations/ directory.
pub async fn run_migrations(pool: &PgPool) -> Result<(), AgentForgeError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| AgentForgeError::DatabaseError(format!("Migration failed: {e}")))
}
