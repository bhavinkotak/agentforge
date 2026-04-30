use agentforge_core::{AgentFile, AgentForgeError, FailureCluster, Result, Scorecard, Trace};
use crate::mutations::{
    inject_few_shot_examples, rewrite_prompt, rewrite_tool_descriptions,
    tighten_output_schema,
};

/// Configuration for the optimizer.
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    pub min_variants: usize,
    pub max_variants: usize,
    /// OpenAI-compatible base URL for LLM-powered mutations
    pub llm_base_url: String,
    pub llm_api_key: String,
    pub llm_model: String,
    /// Minimum number of passing traces before attempting few-shot injection
    pub few_shot_min_traces: usize,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            min_variants: 5,
            max_variants: 20,
            llm_base_url: "https://api.openai.com/v1".to_string(),
            llm_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            llm_model: "gpt-4o".to_string(),
            few_shot_min_traces: 50,
        }
    }
}

/// A candidate agent variant with its mutation description.
#[derive(Debug, Clone)]
pub struct AgentVariant {
    pub agent: AgentFile,
    pub mutation_type: MutationType,
    pub description: String,
    pub parent_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationType {
    PromptRewrite,
    ToolDescriptionRewrite,
    OutputSchemaTighten,
    FewShotInjection,
    InstructionReorder,
}

impl std::fmt::Display for MutationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutationType::PromptRewrite => write!(f, "prompt_rewrite"),
            MutationType::ToolDescriptionRewrite => write!(f, "tool_description_rewrite"),
            MutationType::OutputSchemaTighten => write!(f, "output_schema_tighten"),
            MutationType::FewShotInjection => write!(f, "few_shot_injection"),
            MutationType::InstructionReorder => write!(f, "instruction_reorder"),
        }
    }
}

/// The result of an optimization cycle.
#[derive(Debug)]
pub struct OptimizationResult {
    pub variants: Vec<AgentVariant>,
    pub mutation_types_applied: Vec<MutationType>,
}

/// The optimizer generates candidate variants of an agent.
pub struct Optimizer {
    config: OptimizerConfig,
}

impl Optimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    /// Generate candidate variants based on the current scorecard and failure analysis.
    pub async fn generate_variants(
        &self,
        agent: &AgentFile,
        scorecard: &Scorecard,
        passing_traces: &[Trace],
        parent_sha: &str,
    ) -> Result<OptimizationResult> {
        let mut variants = Vec::new();
        let mut mutation_types = Vec::new();

        tracing::info!(
            agent = %agent.name,
            aggregate_score = scorecard.aggregate_score,
            "Starting optimization cycle"
        );

        // Priority 1: Prompt rewrite (if task completion or instruction adherence is low)
        if scorecard.dimension_scores.task_completion < 0.8
            || scorecard.dimension_scores.instruction_adherence < 0.8
        {
            let n = (self.config.max_variants / 5).max(2);
            match rewrite_prompt(agent, scorecard, &self.config, n).await {
                Ok(mut prompt_variants) => {
                    for pv in prompt_variants.drain(..) {
                        variants.push(AgentVariant {
                            agent: pv,
                            mutation_type: MutationType::PromptRewrite,
                            description: "Prompt rewritten for clarity and constraint tightening".to_string(),
                            parent_sha: parent_sha.to_string(),
                        });
                    }
                    mutation_types.push(MutationType::PromptRewrite);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Prompt rewrite mutation failed");
                    // Apply a deterministic fallback
                    let fallback = reorder_instructions(agent, parent_sha);
                    variants.push(fallback);
                    mutation_types.push(MutationType::InstructionReorder);
                }
            }
        }

        // Priority 2: Tool description rewrite (if tool selection or arg correctness is low)
        if !agent.tools.is_empty()
            && (scorecard.dimension_scores.tool_selection < 0.8
                || scorecard.dimension_scores.argument_correctness < 0.8)
        {
            let n = (self.config.max_variants / 5).max(2);
            match rewrite_tool_descriptions(agent, scorecard, &self.config, n).await {
                Ok(mut tool_variants) => {
                    for tv in tool_variants.drain(..) {
                        variants.push(AgentVariant {
                            agent: tv,
                            mutation_type: MutationType::ToolDescriptionRewrite,
                            description: "Tool descriptions rewritten with examples and type constraints".to_string(),
                            parent_sha: parent_sha.to_string(),
                        });
                    }
                    mutation_types.push(MutationType::ToolDescriptionRewrite);
                }
                Err(e) => tracing::warn!(error = %e, "Tool description rewrite failed"),
            }
        }

        // Priority 3: Output schema tightening (if schema compliance is low)
        if scorecard.dimension_scores.schema_compliance < 0.85 {
            if let Some(schema_variant) = tighten_output_schema(agent) {
                variants.push(AgentVariant {
                    agent: schema_variant,
                    mutation_type: MutationType::OutputSchemaTighten,
                    description: "Output schema tightened with stricter required fields".to_string(),
                    parent_sha: parent_sha.to_string(),
                });
                mutation_types.push(MutationType::OutputSchemaTighten);
            }
        }

        // Priority 4: Few-shot example injection (if enough passing traces)
        if passing_traces.len() >= self.config.few_shot_min_traces {
            match inject_few_shot_examples(agent, passing_traces) {
                Ok(few_shot_variant) => {
                    variants.push(AgentVariant {
                        agent: few_shot_variant,
                        mutation_type: MutationType::FewShotInjection,
                        description: format!(
                            "Injected {} few-shot examples from top-scoring traces",
                            passing_traces.len().min(5)
                        ),
                        parent_sha: parent_sha.to_string(),
                    });
                    mutation_types.push(MutationType::FewShotInjection);
                }
                Err(e) => tracing::warn!(error = %e, "Few-shot injection failed"),
            }
        }

        // Ensure we have at least min_variants
        while variants.len() < self.config.min_variants {
            variants.push(reorder_instructions(agent, parent_sha));
        }

        // Truncate to max_variants
        variants.truncate(self.config.max_variants);

        tracing::info!(
            variants = variants.len(),
            mutations = ?mutation_types.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            "Optimization cycle complete"
        );

        Ok(OptimizationResult {
            variants,
            mutation_types_applied: mutation_types,
        })
    }
}

/// Deterministic fallback: reorder instructions to put critical ones first.
fn reorder_instructions(agent: &AgentFile, parent_sha: &str) -> AgentVariant {
    let mut new_agent = agent.clone();

    // Sort constraints by priority (put "never" constraints first, then "always")
    let mut constraints = new_agent.constraints.clone();
    constraints.sort_by(|a, b| {
        let a_never = a.to_lowercase().starts_with("never");
        let b_never = b.to_lowercase().starts_with("never");
        b_never.cmp(&a_never)
    });
    new_agent.constraints = constraints;

    // Prepend a summary of critical constraints to the system prompt
    if !new_agent.constraints.is_empty() {
        let critical = new_agent.constraints
            .iter()
            .take(3)
            .map(|c| format!("- {}", c))
            .collect::<Vec<_>>()
            .join("\n");

        let new_prompt = format!(
            "CRITICAL RULES (always follow these first):\n{}\n\n{}",
            critical,
            new_agent.system_prompt
        );
        new_agent.system_prompt = new_prompt;
    }

    AgentVariant {
        agent: new_agent,
        mutation_type: MutationType::InstructionReorder,
        description: "Critical instructions moved to the top of the system prompt".to_string(),
        parent_sha: parent_sha.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentforge_core::{
        DimensionScores, EvalWeights, FailureCluster, FailureClusterSummary, ModelConfig,
        ModelProvider, Scorecard,
    };
    use uuid::Uuid;

    fn make_agent() -> AgentFile {
        AgentFile {
            agentforge_schema_version: "1".to_string(),
            name: "test-agent".to_string(),
            version: "1.0.0".to_string(),
            model: ModelConfig {
                provider: ModelProvider::Openai,
                model_id: "gpt-4o".to_string(),
                temperature: Some(0.2),
                max_tokens: Some(2048),
                top_p: None,
            },
            system_prompt: "You are a helpful assistant.\nAlways be polite.".to_string(),
            tools: vec![],
            output_schema: None,
            constraints: vec![
                "Never share personal data.".to_string(),
                "Always confirm before deleting.".to_string(),
            ],
            eval_hints: None,
            metadata: None,
        }
    }

    fn make_scorecard(agg: f64, tc: f64, ts: f64, ac: f64, sc: f64, ia: f64, pe: f64) -> Scorecard {
        Scorecard {
            run_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            agent_name: "test-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            aggregate_score: agg,
            pass_rate: 0.7,
            total_scenarios: 10,
            passed: 7,
            failed: 3,
            errors: 0,
            review_needed: 0,
            dimension_scores: DimensionScores {
                task_completion: tc,
                tool_selection: ts,
                argument_correctness: ac,
                schema_compliance: sc,
                instruction_adherence: ia,
                path_efficiency: pe,
            },
            failure_clusters: vec![],
            duration_seconds: 60,
            total_input_tokens: 5000,
            total_output_tokens: 2000,
        }
    }

    #[tokio::test]
    async fn generates_at_least_min_variants() {
        let agent = make_agent();
        let scorecard = make_scorecard(0.6, 0.5, 0.7, 0.7, 0.5, 0.5, 0.7);
        let config = OptimizerConfig {
            min_variants: 3,
            max_variants: 10,
            llm_api_key: "".to_string(), // No LLM
            llm_model: "gpt-4o".to_string(),
            ..Default::default()
        };
        let optimizer = Optimizer::new(config);
        let result = optimizer
            .generate_variants(&agent, &scorecard, &[], "abc123")
            .await
            .unwrap();
        assert!(
            result.variants.len() >= 3,
            "Expected at least 3 variants, got {}",
            result.variants.len()
        );
    }

    #[tokio::test]
    async fn does_not_exceed_max_variants() {
        let agent = make_agent();
        let scorecard = make_scorecard(0.5, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4);
        let config = OptimizerConfig {
            min_variants: 2,
            max_variants: 5,
            llm_api_key: "".to_string(),
            ..Default::default()
        };
        let optimizer = Optimizer::new(config);
        let result = optimizer
            .generate_variants(&agent, &scorecard, &[], "abc123")
            .await
            .unwrap();
        assert!(result.variants.len() <= 5);
    }

    #[test]
    fn reorder_puts_never_constraints_first() {
        let agent = make_agent();
        let variant = reorder_instructions(&agent, "sha_test");
        assert!(variant.agent.system_prompt.starts_with("CRITICAL RULES"));
        assert!(variant.agent.system_prompt.contains("Never share"));
    }
}
