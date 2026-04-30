use agentforge_core::{
    AgentFile, BenchmarkResult, BenchmarkRun, BenchmarkSuite, BenchmarkTask, Result,
};
use agentforge_runner::{AgentRunner, LlmClient, RunnerConfig};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::normalizer::BenchmarkNormalizer;

/// Configuration for a benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkRunnerConfig {
    pub suite: BenchmarkSuite,
    pub agent_id: Uuid,
    pub runner_config: RunnerConfig,
}

/// Runs an agent against a benchmark suite and produces a `BenchmarkRun`.
pub struct BenchmarkRunner {
    llm: Arc<dyn LlmClient>,
    config: BenchmarkRunnerConfig,
}

impl BenchmarkRunner {
    pub fn new(llm: Arc<dyn LlmClient>, config: BenchmarkRunnerConfig) -> Self {
        Self { llm, config }
    }

    /// Execute the agent on all tasks, score answers, compute accuracy and percentile.
    pub async fn run(
        &self,
        agent: &AgentFile,
        tasks: Vec<BenchmarkTask>,
    ) -> Result<BenchmarkRun> {
        let run_id = Uuid::new_v4();
        let started_at = Utc::now();

        tracing::info!(
            run_id = %run_id,
            suite = %self.config.suite,
            task_count = tasks.len(),
            "Starting benchmark run"
        );

        // Convert tasks to scenarios.
        let scenarios =
            BenchmarkNormalizer::to_scenarios(&tasks, self.config.agent_id);

        let runner = AgentRunner::new(self.llm.clone(), self.config.runner_config.clone());
        let run_result = runner.run(agent, scenarios, None).await;

        // Match traces back to tasks by scenario_id (UUID v5 from task.id).
        let mut results: Vec<BenchmarkResult> = Vec::new();
        for (task, trace) in tasks.iter().zip(run_result.traces.iter()) {
            let agent_answer = trace
                .final_output
                .as_ref()
                .map(|o| o.to_string())
                .unwrap_or_default();

            let mut result = BenchmarkNormalizer::assess_result(task, &agent_answer);
            result.latency_ms = trace.latency_ms;
            results.push(result);
        }

        let total = results.len() as u32;
        let correct = results.iter().filter(|r| r.correct).count() as u32;
        let accuracy = if total == 0 {
            0.0
        } else {
            correct as f64 / total as f64
        };

        let percentile_rank =
            BenchmarkNormalizer::percentile_rank(&self.config.suite, accuracy);

        Ok(BenchmarkRun {
            id: run_id,
            agent_id: self.config.agent_id,
            suite: self.config.suite.clone(),
            total_tasks: total,
            correct,
            accuracy,
            percentile_rank,
            results,
            started_at,
            completed_at: Some(Utc::now()),
        })
    }
}
