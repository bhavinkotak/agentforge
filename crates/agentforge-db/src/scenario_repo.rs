use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use agentforge_core::{
    AgentForgeError, Scenario, ScenarioInput, ScenarioExpected,
    DifficultyTier, ScenarioSource, ConversationTurn, ConversationRole,
    ExpectedToolCall, Result,
};
use crate::db_err;

pub struct ScenarioRepo {
    pool: PgPool,
}

impl ScenarioRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_batch(&self, scenarios: &[Scenario]) -> Result<Vec<Uuid>> {
        let mut ids = Vec::new();
        for s in scenarios {
            let id = self.insert(s).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    pub async fn insert(&self, s: &Scenario) -> Result<Uuid> {
        let input_json = serde_json::to_value(&s.input)
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;
        let expected_json = serde_json::to_value(&s.expected)
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;
        let diff_str = s.difficulty.to_string();
        let source_str = s.source.to_string();

        sqlx::query(
            r#"
            INSERT INTO scenarios (id, agent_id, input, expected, difficulty, domain, source, tags, created_at)
            VALUES ($1, $2, $3, $4, $5::difficulty_tier, $6, $7::scenario_source, $8, $9)
            "#,
        )
        .bind(s.id)
        .bind(s.agent_id)
        .bind(input_json)
        .bind(expected_json)
        .bind(diff_str)
        .bind(s.domain.clone())
        .bind(source_str)
        .bind(&s.tags)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(s.id)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Scenario> {
        let row = sqlx::query!(
            r#"
            SELECT id, agent_id, input, expected,
                   difficulty as "difficulty: String",
                   domain,
                   source as "source: String",
                   tags, created_at
            FROM scenarios WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AgentForgeError::NotFound { resource: "Scenario", id: id.to_string() })?;

        self.row_to_scenario(row.id, row.agent_id, row.input, row.expected,
            row.difficulty, row.domain, row.source, row.tags, row.created_at)
    }

    pub async fn list_by_agent(&self, agent_id: Uuid, limit: i64) -> Result<Vec<Scenario>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, agent_id, input, expected,
                   difficulty as "difficulty: String",
                   domain,
                   source as "source: String",
                   tags, created_at
            FROM scenarios
            WHERE agent_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            agent_id,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.into_iter()
            .map(|r| self.row_to_scenario(r.id, r.agent_id, r.input, r.expected,
                r.difficulty, r.domain, r.source, r.tags, r.created_at))
            .collect()
    }

    fn row_to_scenario(
        &self,
        id: Uuid,
        agent_id: Uuid,
        input: serde_json::Value,
        expected: serde_json::Value,
        difficulty: String,
        domain: Option<String>,
        source: String,
        tags: Vec<String>,
        created_at: chrono::DateTime<Utc>,
    ) -> Result<Scenario> {
        let parsed_input: ScenarioInput = serde_json::from_value(input)
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;
        let parsed_expected: ScenarioExpected = serde_json::from_value(expected)
            .map_err(|e| AgentForgeError::SerializationError(e.to_string()))?;

        Ok(Scenario {
            id,
            agent_id,
            input: parsed_input,
            expected: parsed_expected,
            difficulty: parse_difficulty(&difficulty),
            domain,
            source: parse_source(&source),
            tags,
            created_at,
        })
    }
}

fn parse_difficulty(s: &str) -> DifficultyTier {
    match s {
        "easy" => DifficultyTier::Easy,
        "hard" => DifficultyTier::Hard,
        "edge" => DifficultyTier::Edge,
        _ => DifficultyTier::Medium,
    }
}

fn parse_source(s: &str) -> ScenarioSource {
    match s {
        "adversarial" => ScenarioSource::Adversarial,
        "domain_seeded" => ScenarioSource::DomainSeeded,
        "manual" => ScenarioSource::Manual,
        _ => ScenarioSource::SchemaDerived,
    }
}
