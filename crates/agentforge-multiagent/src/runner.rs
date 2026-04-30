use agentforge_core::{AgentGraph, Result, Scenario};
use agentforge_runner::{AgentRunner, LlmClient, RunnerConfig};
use std::collections::HashMap;
use std::sync::Arc;

use crate::graph::GraphRunResult;

/// Executes an `AgentGraph` against a set of scenarios in topological order.
///
/// Each node receives the output of its predecessor nodes as additional context
/// in `ScenarioInput::context`, merged under their respective `input_key`s.
pub struct MultiAgentRunner {
    llm: Arc<dyn LlmClient>,
    config: RunnerConfig,
}

impl MultiAgentRunner {
    pub fn new(llm: Arc<dyn LlmClient>, config: RunnerConfig) -> Self {
        Self { llm, config }
    }

    /// Run the graph sequentially by topological order and collect per-node traces.
    pub async fn run(
        &self,
        graph: &AgentGraph,
        scenarios: Vec<Scenario>,
    ) -> Result<GraphRunResult> {
        let order = graph
            .topological_order()
            .map_err(|e| agentforge_core::AgentForgeError::ConfigError(e))?;

        let mut node_traces: HashMap<String, Vec<agentforge_core::Trace>> = HashMap::new();
        // Stores last output per scenario per node for context threading.
        // Key: (node_id, scenario_id) → final output JSON
        let mut outputs: HashMap<(String, uuid::Uuid), serde_json::Value> = HashMap::new();

        for node in order {
            tracing::info!(node_id = %node.id, role = %node.role, "Running graph node");

            // Enrich each scenario's context with upstream outputs.
            let enriched_scenarios: Vec<Scenario> = scenarios
                .iter()
                .map(|s| {
                    let mut enriched = s.clone();
                    let mut ctx = enriched.input.context.take().unwrap_or(serde_json::Value::Object(Default::default()));

                    // Find edges pointing to this node and inject their upstream outputs.
                    for edge in &graph.edges {
                        if edge.to != node.id {
                            continue;
                        }
                        let key = (edge.from.clone(), s.id);
                        if let Some(upstream_output) = outputs.get(&key) {
                            let ctx_key = edge
                                .input_key
                                .clone()
                                .unwrap_or_else(|| edge.from.clone());
                            if let Some(obj) = ctx.as_object_mut() {
                                obj.insert(ctx_key, upstream_output.clone());
                            }
                        }
                    }

                    enriched.input.context = Some(ctx);
                    enriched
                })
                .collect();

            let runner =
                AgentRunner::new(self.llm.clone(), self.config.clone());
            let result = runner.run(&node.agent, enriched_scenarios, None).await;

            // Capture outputs for downstream nodes.
            for trace in &result.traces {
                if let Some(output) = &trace.final_output {
                    outputs.insert((node.id.clone(), trace.scenario_id), output.clone());
                }
            }

            node_traces.insert(node.id.clone(), result.traces);
        }

        Ok(GraphRunResult::from_traces(graph.id, node_traces))
    }
}
