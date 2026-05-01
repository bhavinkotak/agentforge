use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AgentFile;

/// A node in a multi-agent graph — an agent with a role label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    /// Unique node identifier within the graph.
    pub id: String,
    /// Human-readable role (e.g. "planner", "executor", "critic").
    pub role: String,
    /// The agent's full specification.
    pub agent: AgentFile,
}

/// A directed edge connecting two nodes: the output field of `from` feeds the input of `to`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// ID of the source node.
    pub from: String,
    /// ID of the destination node.
    pub to: String,
    /// The field name in `from`'s output schema that is passed.
    pub output_field: Option<String>,
    /// The context key under which it is injected into `to`'s input.
    pub input_key: Option<String>,
}

/// A directed acyclic graph of agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGraph {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<AgentNode>,
    pub edges: Vec<GraphEdge>,
}

impl AgentGraph {
    /// Returns nodes in topological order (Kahn's algorithm).
    /// Returns `Err` if the graph contains a cycle.
    pub fn topological_order(&self) -> Result<Vec<&AgentNode>, String> {
        use std::collections::{HashMap, VecDeque};

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(node.id.as_str()).or_insert(0);
            adj.entry(node.id.as_str()).or_default();
        }

        for edge in &self.edges {
            *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
            adj.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let node_by_id: HashMap<&str, &AgentNode> =
            self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        let mut result = Vec::new();
        while let Some(id) = queue.pop_front() {
            if let Some(node) = node_by_id.get(id) {
                result.push(*node);
            }
            if let Some(neighbors) = adj.get(id) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err("AgentGraph contains a cycle".to_string());
        }

        Ok(result)
    }
}

/// Composite scoring result for a multi-agent graph run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentScorecard {
    pub graph_id: Uuid,
    /// Per-node aggregate scores (keyed by node id).
    pub node_scores: std::collections::HashMap<String, f64>,
    /// Weighted average across all nodes.
    pub composite_score: f64,
    pub pass_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph(edges: &[(&str, &str)]) -> AgentGraph {
        use crate::{AgentFile, ModelConfig, ModelProvider};
        let node_ids: std::collections::HashSet<&str> =
            edges.iter().flat_map(|(a, b)| [*a, *b]).collect();

        let nodes = node_ids
            .into_iter()
            .map(|id| AgentNode {
                id: id.to_string(),
                role: id.to_string(),
                agent: AgentFile {
                    agentforge_schema_version: "1".into(),
                    name: id.to_string(),
                    version: "0.1.0".into(),
                    model: ModelConfig {
                        provider: ModelProvider::Openai,
                        model_id: "gpt-4o".into(),
                        temperature: None,
                        max_tokens: None,
                        top_p: None,
                    },
                    system_prompt: String::new(),
                    tools: vec![],
                    output_schema: None,
                    constraints: vec![],
                    eval_hints: None,
                    metadata: None,
                },
            })
            .collect();

        let graph_edges = edges
            .iter()
            .map(|(from, to)| GraphEdge {
                from: from.to_string(),
                to: to.to_string(),
                output_field: None,
                input_key: None,
            })
            .collect();

        AgentGraph {
            id: Uuid::new_v4(),
            name: "test".into(),
            description: None,
            nodes,
            edges: graph_edges,
        }
    }

    #[test]
    fn topological_order_linear() {
        let g = make_graph(&[("a", "b"), ("b", "c")]);
        let order = g.topological_order().unwrap();
        let ids: Vec<&str> = order.iter().map(|n| n.id.as_str()).collect();
        // 'a' must come before 'b', 'b' before 'c'
        assert!(ids.iter().position(|&x| x == "a") < ids.iter().position(|&x| x == "b"));
        assert!(ids.iter().position(|&x| x == "b") < ids.iter().position(|&x| x == "c"));
    }
}
