use agentforge_core::{
    BenchmarkBaseline, BenchmarkResult, BenchmarkSuite, BenchmarkTask, DifficultyTier,
    ExpectedToolCall, Scenario, ScenarioExpected, ScenarioInput, ScenarioSource,
    published_baselines,
};
use chrono::Utc;
use uuid::Uuid;

/// Converts benchmark tasks to AgentForge `Scenario`s and computes percentile ranks.
pub struct BenchmarkNormalizer;

impl BenchmarkNormalizer {
    /// Convert a slice of benchmark tasks into AgentForge `Scenario`s.
    pub fn to_scenarios(tasks: &[BenchmarkTask], agent_id: Uuid) -> Vec<Scenario> {
        tasks.iter().map(|t| task_to_scenario(t, agent_id)).collect()
    }

    /// Compute percentile rank for the agent vs. published baselines for a suite.
    ///
    /// Returns `None` if there are no published baselines for the suite.
    pub fn percentile_rank(suite: &BenchmarkSuite, agent_accuracy: f64) -> Option<f64> {
        let all_baselines = published_baselines();
        let baselines: Vec<&BenchmarkBaseline> = all_baselines
            .iter()
            .filter(|b| &b.suite == suite)
            .collect();

        if baselines.is_empty() {
            return None;
        }

        let scores_below = baselines
            .iter()
            .filter(|b| agent_accuracy > b.accuracy)
            .count();

        let percentile = (scores_below as f64 / baselines.len() as f64) * 100.0;
        Some(percentile.clamp(0.0, 100.0))
    }

    /// Assess a single result: compare agent_answer to expected_answer (exact / substring match).
    pub fn assess_result(task: &BenchmarkTask, agent_answer: &str) -> BenchmarkResult {
        let correct = match &task.expected_answer {
            None => false,
            Some(expected) => {
                // Case-insensitive substring match
                agent_answer
                    .to_lowercase()
                    .contains(&expected.to_lowercase())
            }
        };

        BenchmarkResult {
            task_id: task.id.clone(),
            suite: task.suite.clone(),
            agent_answer: Some(agent_answer.to_string()),
            correct,
            score: if correct { 1.0 } else { 0.0 },
            latency_ms: 0,
            token_cost_usd: 0.0,
        }
    }
}

fn task_to_scenario(task: &BenchmarkTask, agent_id: Uuid) -> Scenario {
    let difficulty = match task.difficulty_level {
        Some(1) => DifficultyTier::Easy,
        Some(2) => DifficultyTier::Medium,
        Some(3) => DifficultyTier::Hard,
        _ => DifficultyTier::Medium,
    };

    Scenario {
        id: Uuid::new_v5(&Uuid::NAMESPACE_DNS, task.id.as_bytes()),
        agent_id,
        input: ScenarioInput {
            user_message: task.question.clone(),
            conversation_history: vec![],
            context: if task.context_files.is_empty() {
                None
            } else {
                Some(serde_json::json!({ "context_files": task.context_files }))
            },
        },
        expected: ScenarioExpected {
            tool_calls: vec![],
            output_schema: None,
            pass_criteria: task
                .expected_answer
                .as_deref()
                .map(|a| format!("The final answer must contain: {a}"))
                .unwrap_or_else(|| "Agent completes the task.".to_string()),
            min_turns: None,
            max_turns: Some(10),
        },
        difficulty,
        domain: Some(task.suite.to_string()),
        source: ScenarioSource::Manual,
        tags: vec![
            "benchmark".to_string(),
            task.suite.to_string(),
            task.id.clone(),
        ],
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assess_correct_answer() {
        let task = BenchmarkTask {
            id: "t1".into(),
            suite: BenchmarkSuite::Gaia,
            difficulty_level: Some(1),
            question: "What is 2+2?".into(),
            expected_answer: Some("4".into()),
            context_files: vec![],
        };
        let result = BenchmarkNormalizer::assess_result(&task, "The answer is 4.");
        assert!(result.correct);
    }

    #[test]
    fn assess_wrong_answer() {
        let task = BenchmarkTask {
            id: "t2".into(),
            suite: BenchmarkSuite::Gaia,
            difficulty_level: None,
            question: "What is 2+2?".into(),
            expected_answer: Some("4".into()),
            context_files: vec![],
        };
        let result = BenchmarkNormalizer::assess_result(&task, "Five.");
        assert!(!result.correct);
    }
}
