use crate::db_err;
use agentforge_core::{AgentForgeError, ExportFormat, ExportStatus, FineTuneExport, Result};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
struct FineTuneRow {
    id: Uuid,
    run_id: Uuid,
    format: String,
    status: String,
    row_count: Option<i32>,
    file_path: Option<String>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl FineTuneRow {
    fn into_export(self) -> FineTuneExport {
        FineTuneExport {
            id: self.id,
            run_id: self.run_id,
            format: parse_format(&self.format),
            status: parse_status(&self.status),
            row_count: self.row_count.map(|n| n as u32),
            file_path: self.file_path,
            error_message: self.error_message,
            created_at: self.created_at,
            completed_at: self.completed_at,
        }
    }
}

pub struct FineTuneRepo {
    pool: PgPool,
}

impl FineTuneRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, export: &FineTuneExport) -> Result<FineTuneExport> {
        let format_str = export.format.to_string();
        let status_str = export.status.to_string();

        sqlx::query(
            "INSERT INTO finetune_exports \
                (id, run_id, format, status, row_count, file_path, error_message, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(export.id)
        .bind(export.run_id)
        .bind(&format_str)
        .bind(&status_str)
        .bind(export.row_count.map(|n| n as i32))
        .bind(&export.file_path)
        .bind(&export.error_message)
        .bind(export.created_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        self.find_by_id(export.id).await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<FineTuneExport> {
        sqlx::query_as::<_, FineTuneRow>(
            "SELECT id, run_id, format, status, row_count, \
                    file_path, error_message, created_at, completed_at \
             FROM finetune_exports WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AgentForgeError::NotFound {
            resource: "FineTuneExport",
            id: id.to_string(),
        })
        .map(|r| r.into_export())
    }

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &ExportStatus,
        row_count: Option<u32>,
        file_path: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let status_str = status.to_string();
        let completed_at =
            if *status == ExportStatus::Complete || *status == ExportStatus::Error {
                Some(Utc::now())
            } else {
                None
            };

        sqlx::query(
            "UPDATE finetune_exports \
             SET status        = $2, \
                 row_count     = $3, \
                 file_path     = $4, \
                 error_message = $5, \
                 completed_at  = $6 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(&status_str)
        .bind(row_count.map(|n| n as i32))
        .bind(file_path)
        .bind(error_message)
        .bind(completed_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }
}

fn parse_format(s: &str) -> ExportFormat {
    match s {
        "anthropic" => ExportFormat::Anthropic,
        "huggingface" => ExportFormat::HuggingFace,
        _ => ExportFormat::OpenAi,
    }
}

fn parse_status(s: &str) -> ExportStatus {
    match s {
        "running" => ExportStatus::Running,
        "complete" => ExportStatus::Complete,
        "error" => ExportStatus::Error,
        _ => ExportStatus::Pending,
    }
}
