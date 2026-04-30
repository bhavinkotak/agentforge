use agentforge_core::{
    AgentFile, ConversationRole, ConversationTurn, DifficultyTier, ExpectedToolCall,
    Result, Scenario, ScenarioExpected, ScenarioInput, ScenarioSource, ToolDefinition,
};
use uuid::Uuid;
use chrono::Utc;

/// Generate scenarios that exercise every tool and output field.
pub fn generate_schema_derived_scenarios(
    agent: &AgentFile,
    count: usize,
    agent_id: Uuid,
) -> Result<Vec<Scenario>> {
    let mut scenarios = Vec::new();

    // 1. One happy-path scenario per tool
    for tool in &agent.tools {
        if scenarios.len() >= count {
            break;
        }
        scenarios.push(happy_path_for_tool(agent, tool, agent_id));
    }

    // 2. If we have output schema, generate scenarios for required fields
    if let Some(schema) = &agent.output_schema {
        if scenarios.len() < count {
            if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                for field in required {
                    if scenarios.len() >= count {
                        break;
                    }
                    if let Some(field_name) = field.as_str() {
                        scenarios.push(output_field_scenario(agent, field_name, agent_id));
                    }
                }
            }
        }
    }

    // 3. Fill remaining with multi-tool scenarios
    let mut rng_seed = 42u64;
    while scenarios.len() < count && !agent.tools.is_empty() {
        scenarios.push(multi_tool_scenario(agent, agent_id, rng_seed));
        rng_seed += 1;
    }

    // If still not enough (agent has no tools), generate generic task scenarios
    while scenarios.len() < count {
        scenarios.push(generic_task_scenario(agent, agent_id, scenarios.len()));
    }

    Ok(scenarios)
}

fn happy_path_for_tool(agent: &AgentFile, tool: &ToolDefinition, agent_id: Uuid) -> Scenario {
    let user_message = format!(
        "Please use the {} tool to help me with my request.",
        tool.name
    );

    let pass_criteria = format!(
        "The agent should call the '{}' tool with valid arguments and provide a clear response.",
        tool.name
    );

    Scenario {
        id: Uuid::new_v4(),
        agent_id,
        input: ScenarioInput {
            user_message,
            conversation_history: vec![],
            context: None,
        },
        expected: ScenarioExpected {
            tool_calls: vec![ExpectedToolCall {
                tool_name: tool.name.clone(),
                required: true,
                argument_schema: Some(tool.parameters.clone()),
            }],
            output_schema: agent.output_schema.clone(),
            pass_criteria,
            min_turns: Some(1),
            max_turns: Some(5),
        },
        difficulty: DifficultyTier::Easy,
        domain: agent.eval_hints.as_ref().and_then(|h| h.domain.clone()),
        source: ScenarioSource::SchemaDerived,
        tags: vec!["happy_path".to_string(), format!("tool:{}", tool.name)],
        created_at: Utc::now(),
    }
}

fn output_field_scenario(agent: &AgentFile, field_name: &str, agent_id: Uuid) -> Scenario {
    Scenario {
        id: Uuid::new_v4(),
        agent_id,
        input: ScenarioInput {
            user_message: format!(
                "Complete a task that requires the agent to produce a '{}' field in its output.",
                field_name
            ),
            conversation_history: vec![],
            context: None,
        },
        expected: ScenarioExpected {
            tool_calls: vec![],
            output_schema: agent.output_schema.clone(),
            pass_criteria: format!(
                "The agent's output must include a valid '{}' field matching the schema.",
                field_name
            ),
            min_turns: Some(1),
            max_turns: Some(3),
        },
        difficulty: DifficultyTier::Easy,
        domain: agent.eval_hints.as_ref().and_then(|h| h.domain.clone()),
        source: ScenarioSource::SchemaDerived,
        tags: vec!["output_field".to_string(), format!("field:{}", field_name)],
        created_at: Utc::now(),
    }
}

fn multi_tool_scenario(agent: &AgentFile, agent_id: Uuid, seed: u64) -> Scenario {
    // Pick two tools (or repeat if only one)
    let n = agent.tools.len();
    let tool_a = &agent.tools[(seed as usize) % n];
    let tool_b = &agent.tools[(seed as usize + 1) % n];

    let expected_calls = if tool_a.name == tool_b.name {
        vec![ExpectedToolCall {
            tool_name: tool_a.name.clone(),
            required: true,
            argument_schema: Some(tool_a.parameters.clone()),
        }]
    } else {
        vec![
            ExpectedToolCall {
                tool_name: tool_a.name.clone(),
                required: true,
                argument_schema: Some(tool_a.parameters.clone()),
            },
            ExpectedToolCall {
                tool_name: tool_b.name.clone(),
                required: false,
                argument_schema: Some(tool_b.parameters.clone()),
            },
        ]
    };

    Scenario {
        id: Uuid::new_v4(),
        agent_id,
        input: ScenarioInput {
            user_message: format!(
                "I need help with a task that may require using both {} and {} tools.",
                tool_a.name, tool_b.name
            ),
            conversation_history: vec![],
            context: None,
        },
        expected: ScenarioExpected {
            tool_calls: expected_calls,
            output_schema: agent.output_schema.clone(),
            pass_criteria: "Agent should call the appropriate tools in a logical order and provide a complete response.".to_string(),
            min_turns: Some(1),
            max_turns: Some(6),
        },
        difficulty: DifficultyTier::Medium,
        domain: agent.eval_hints.as_ref().and_then(|h| h.domain.clone()),
        source: ScenarioSource::SchemaDerived,
        tags: vec!["multi_tool".to_string()],
        created_at: Utc::now(),
    }
}

fn generic_task_scenario(agent: &AgentFile, agent_id: Uuid, index: usize) -> Scenario {
    let messages = vec![
        "Please help me with my request.",
        "I need assistance with a task.",
        "Can you help me?",
        "I have a question for you.",
        "Please assist me.",
    ];
    let msg = messages[index % messages.len()];

    Scenario {
        id: Uuid::new_v4(),
        agent_id,
        input: ScenarioInput {
            user_message: msg.to_string(),
            conversation_history: vec![],
            context: None,
        },
        expected: ScenarioExpected {
            tool_calls: vec![],
            output_schema: agent.output_schema.clone(),
            pass_criteria: "Agent should provide a helpful and relevant response.".to_string(),
            min_turns: Some(1),
            max_turns: Some(5),
        },
        difficulty: DifficultyTier::Easy,
        domain: agent.eval_hints.as_ref().and_then(|h| h.domain.clone()),
        source: ScenarioSource::SchemaDerived,
        tags: vec!["generic".to_string()],
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentforge_core::{EvalHints, ModelConfig, ModelProvider};

    fn make_agent_with_tools() -> AgentFile {
        AgentFile {
            agentforge_schema_version: "1".to_string(),
            name: "test-agent".to_string(),
            version: "1.0.0".to_string(),
            model: ModelConfig {
                provider: ModelProvider::Openai,
                model_id: "gpt-4o".to_string(),
                temperature: None,
                max_tokens: None,
                top_p: None,
            },
            system_prompt: "You are helpful.".to_string(),
            tools: vec![
                ToolDefinition {
                    name: "tool_a".to_string(),
                    description: "Tool A".to_string(),
                    parameters: serde_json::json!({"type": "object", "properties": {}}),
                },
                ToolDefinition {
                    name: "tool_b".to_string(),
                    description: "Tool B".to_string(),
                    parameters: serde_json::json!({"type": "object", "properties": {}}),
                },
            ],
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {"response": {"type": "string"}},
                "required": ["response"]
            })),
            constraints: vec![],
            eval_hints: None,
            metadata: None,
        }
    }

    #[test]
    fn generates_at_least_one_per_tool() {
        let agent = make_agent_with_tools();
        let id = Uuid::new_v4();
        let scenarios = generate_schema_derived_scenarios(&agent, 5, id).unwrap();
        assert_eq!(scenarios.len(), 5);
        // At least one scenario should target tool_a
        assert!(scenarios.iter().any(|s| s.tags.iter().any(|t| t.contains("tool_a"))));
    }

    #[test]
    fn generates_exact_count() {
        let agent = make_agent_with_tools();
        let id = Uuid::new_v4();
        for n in [1, 5, 10, 20] {
            let s = generate_schema_derived_scenarios(&agent, n, id).unwrap();
            assert_eq!(s.len(), n, "Expected {n} scenarios");
        }
    }

    #[test]
    fn all_scenarios_have_unique_ids() {
        let agent = make_agent_with_tools();
        let id = Uuid::new_v4();
        let scenarios = generate_schema_derived_scenarios(&agent, 10, id).unwrap();
        let ids: std::collections::HashSet<_> = scenarios.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), 10);
    }
}
