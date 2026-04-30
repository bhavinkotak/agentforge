use crate::db_err;
use agentforge_core::{
    AgentForgeError, BenchmarkResult, BenchmarkRun, BenchmarkSuite, Result,
};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
struct BenchmarkRunRow {
    id: Uuid,
    agent_id: Uuid,
    suite: String,
    total_tasks: i32,
    correct: i32,
    accuracy: Option<f64>,
    percentile_rank: Option<f64>,
    #[allow(dead_code)]
    status: String,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

impl BenchmarkRunRow {
    fn into_run(self) -> BenchmarkRun {
        BenchmarkRun {
            id: self.id,
            agent_id: self.agent_id,
            suite: parse_suite(&self.suite),
            total_tasks: self.total_tasks as u32,
            correct: self.correct as u32,
            accuracy: self.accuracy.unwrap_or(0.0),
            percentile_rank: self.percentile_rank,
            results: vec![],
            started_at: self.started_at.unwrap_or(self.created_at),
            completed_at: self.completed_at,
        }
    }
}

pub struct BenchmarkRepo {
    pool: PgPool,
}

impl BenchmarkRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_run(&self, run: &BenchmarkRun) -> Result<BenchmarkRun> {
        let suite_str = run.suite.to_string();

        sqlx::query(
            "INSERT INTO benchmark_runs \
                (id, agent_id, suite, total_tasks, correct, accuracy, \
                 percentile_rank, status, created_at, started_at, completed_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', $8, $9, $10)",
        )
        .bind(run.id)
        .bind(run.agent_id)
        .bind(&suite_str)
        .bind(run.total_tasks as i32)
        .bind(run.correct as i32)
        .bind(run.accuracy)
        .bind(run.percentile_rank)
        .bind(run.started_at)
        .bind(run.started_at)
        .bind(run.completed_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        for result in &run.results {
            self.insert_result(run.id, result).await?;
        }

        self.find_run_by_id(run.id).await
    }

    pub async fn insert_result(&self, run_id: Uuid, result: &BenchmarkResult) -> Result<()> {
        let suite_str = result.suite.to_string();

        sqlx::query(
            "INSERT INTO benchmark_results \
                (id, benchmark_run_id, task_id, suite, agent_answer, \
                 expected_answer, correct, score, latency_ms, token_cost_usd, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(Uuid::new_v4())
        .bind(run_id)
        .bind(&result.task_id)
        .bind(&suite_str)
        .bind(&result.agent_answer)
        .bind(None::<String>)
        .bind(result.correct)
        .bind(result.score)
        .bind(result.latency_ms as i64)
        .bind(result.token_cost_usd)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    pub async fn find_run_by_id(&self, id: Uuid) -> Result<BenchmarkRun> {
        sqlx::query_as::<_, BenchmarkRunRow>(
            "SELECT id, agent_id, suite, total_tasks, correct, accuracy, \
                    percentile_rank, status, created_at, started_at, completed_at \
             FROM benchmark_runs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AgentForgeError::NotFound {
            resource: "BenchmarkRun",
            id: id.to_string(),
        })
        .map(|r| r.into_run())
    }

    pub async fn update_run_complete(
        &self,
        id: Uuid,
        total_tasks: u32,
        correct: u32,
        accuracy: f64,
        percentile_rank: Option<f64>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE benchmark_runs \
             SET total_tasks     = $2, \
                 correct         = $3, \
                 accuracy        = $4, \
                 percentile_rank = $5, \
                 status          = 'complete', \
                 completed_at    = $6 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(total_tasks as i32)
        .bind(correct as i32)
        .bind(accuracy)
        .bind(percentile_rank)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    pub async fn list_by_agent(&self, agent_id: Uuid) -> Result<Vec<BenchmarkRun>> {
        let rows = sqlx::query_as::<_, BenchmarkRunRow>(
            "SELECT id, agent_id, suite, total_tasks, correct, accuracy, \
                    percentile_rank, status, created_at, started_at, completed_at \
             FROM benchmark_runs WHERE agent_id = $1 ORDER BY created_at DESC",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| r.into_run()).collect())
    }
}

fn parse_suite(s: &str) -> BenchmarkSuite {
    match s {
        "agentbench" => BenchmarkSuite::AgentBench,
        "webarena" => BenchmarkSuite::WebArena,
        _ => BenchmarkSuite::Gaia,
    }
}
