use crate::db_err;
use agentforge_core::{AgentForgeError, Result, ShadowComparison, ShadowRun, ShadowRunStatus};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
struct ShadowRunRow {
    id: Uuid,
    champion_agent_id: Uuid,
    candidate_agent_id: Uuid,
    traffic_percent: i16,
    status: String,
    comparison_result: Option<serde_json::Value>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

impl ShadowRunRow {
    fn into_shadow_run(self) -> Result<ShadowRun> {
        let status = match self.status.as_str() {
            "pending" => ShadowRunStatus::Pending,
            "running" => ShadowRunStatus::Running,
            "complete" => ShadowRunStatus::Complete,
            _ => ShadowRunStatus::Error,
        };
        let comparison: Option<ShadowComparison> = self
            .comparison_result
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;
        Ok(ShadowRun {
            id: self.id,
            champion_agent_id: self.champion_agent_id,
            candidate_agent_id: self.candidate_agent_id,
            traffic_percent: self.traffic_percent as u8,
            status,
            comparison,
            error_message: self.error_message,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
        })
    }
}

pub struct ShadowRepo {
    pool: PgPool,
}

impl ShadowRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, run: &ShadowRun) -> Result<ShadowRun> {
        let status_str = run.status.to_string();
        let comparison_json = run
            .comparison
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO shadow_runs \
                (id, champion_agent_id, candidate_agent_id, traffic_percent, \
                 status, comparison_result, error_message, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(run.id)
        .bind(run.champion_agent_id)
        .bind(run.candidate_agent_id)
        .bind(run.traffic_percent as i16)
        .bind(&status_str)
        .bind(&comparison_json)
        .bind(&run.error_message)
        .bind(run.created_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        self.find_by_id(run.id).await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<ShadowRun> {
        sqlx::query_as::<_, ShadowRunRow>(
            "SELECT id, champion_agent_id, candidate_agent_id, traffic_percent, \
                    status, comparison_result, error_message, \
                    created_at, started_at, completed_at \
             FROM shadow_runs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AgentForgeError::NotFound {
            resource: "ShadowRun",
            id: id.to_string(),
        })?
        .into_shadow_run()
    }

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &ShadowRunStatus,
        comparison: Option<&ShadowComparison>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let status_str = status.to_string();
        let comparison_json = comparison
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;
        let completed_at: Option<DateTime<Utc>> =
            if *status == ShadowRunStatus::Complete || *status == ShadowRunStatus::Error {
                Some(Utc::now())
            } else {
                None
            };
        let started_at: Option<DateTime<Utc>> = if *status == ShadowRunStatus::Running {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            "UPDATE shadow_runs \
             SET status            = $2, \
                 comparison_result = $3, \
                 error_message     = $4, \
                 started_at        = COALESCE(started_at, $5), \
                 completed_at      = $6 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(&status_str)
        .bind(&comparison_json)
        .bind(error_message)
        .bind(started_at)
        .bind(completed_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ShadowRun>> {
        let rows = sqlx::query_as::<_, ShadowRunRow>(
            "SELECT id, champion_agent_id, candidate_agent_id, traffic_percent, \
                    status, comparison_result, error_message, \
                    created_at, started_at, completed_at \
             FROM shadow_runs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.into_iter().map(|r| r.into_shadow_run()).collect()
    }
}
