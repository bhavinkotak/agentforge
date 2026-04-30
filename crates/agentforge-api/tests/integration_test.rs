use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use agentforge_core::{AgentFileFormat, ModelProvider};
use agentforge_parser::{parse_agent_file, to_agent_version, validate_agent_file};
use agentforge_scenarios::{generate_scenarios, ScenarioGeneratorConfig};

const SAMPLE_AGENT_YAML: &str = r#"
agentforge_schema_version: "1"
name: test-agent
version: "1.0.0"
model:
  provider: openai
  model_id: gpt-4o
  temperature: 0.2
system_prompt: "You are a helpful assistant. Answer questions concisely."
tools:
  - name: search
    description: "Search for information on the internet"
    parameters:
      type: object
      properties:
        query:
          type: string
          description: "The search query"
      required: [query]
output_schema:
  type: object
  properties:
    response:
      type: string
  required: [response]
constraints:
  - "Never provide harmful information"
eval_hints:
  scenario_count: 10
  pass_threshold: 0.75
"#;

const SAMPLE_COPILOT_AGENT_MD: &str = r#"---
name: 'Code Review Expert'
description: 'Specialist in reviewing code for security, performance, and maintainability'
model: GPT-4.1
tools: ['read', 'search/codebase', 'github/*']
---

# Code Review Expert

You are an expert code reviewer specializing in security, performance, and maintainability.

## Review Focus Areas

- **Security**: Check for injection vulnerabilities, authentication issues, and data exposure
- **Performance**: Identify N+1 queries, unnecessary allocations, and blocking operations
- **Maintainability**: Evaluate code clarity, test coverage, and adherence to SOLID principles

## Behavioral Constraints

- Always explain the reason behind each suggestion
- Provide concrete code examples when recommending changes
- Prioritize security issues above all others
"#;

// ─── Parser tests (no external dependencies) ────────────────────────────────

#[test]
fn parse_sample_agent_yaml() {
    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    assert_eq!(parsed.agent.name, "test-agent");
    assert_eq!(parsed.agent.version, "1.0.0");
    assert_eq!(parsed.agent.tools.len(), 1);
    assert!(!parsed.sha.is_empty());
}

#[test]
fn validate_sample_agent_passes() {
    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    let result = validate_agent_file(&parsed.agent);
    assert!(
        result.errors.is_empty(),
        "Unexpected errors: {:?}",
        result.errors
    );
}

#[test]
fn to_agent_version_produces_sha() {
    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    let version = to_agent_version(parsed);
    assert!(!version.sha.is_empty());
    assert_eq!(version.name, "test-agent");
}

#[test]
fn parse_copilot_agent_md_format() {
    let parsed = parse_agent_file(SAMPLE_COPILOT_AGENT_MD).unwrap();
    assert_eq!(parsed.format, AgentFileFormat::CopilotAgentMd);
    assert_eq!(parsed.agent.name, "Code Review Expert");
    assert_eq!(parsed.agent.model.model_id, "GPT-4.1");
    assert_eq!(parsed.agent.model.provider, ModelProvider::Openai);
    // System prompt is the Markdown body
    assert!(parsed.agent.system_prompt.contains("Code Review Expert"));
    assert!(parsed.agent.system_prompt.contains("Security"));
    // Tools are mapped from capability references
    assert_eq!(parsed.agent.tools.len(), 3);
    assert_eq!(parsed.agent.tools[0].name, "read");
    assert_eq!(parsed.agent.tools[1].name, "codebase");
    assert_eq!(parsed.agent.tools[2].name, "github");
}

#[test]
fn copilot_agent_md_description_in_metadata() {
    let parsed = parse_agent_file(SAMPLE_COPILOT_AGENT_MD).unwrap();
    let meta = parsed.agent.metadata.expect("should have metadata");
    assert_eq!(
        meta["description"].as_str().unwrap(),
        "Specialist in reviewing code for security, performance, and maintainability"
    );
}

#[test]
fn parse_copilot_agent_md_fixture_file() {
    let content = include_str!("../../../fixtures/agentforge-evaluator.agent.md");
    let parsed = parse_agent_file(content).unwrap();
    assert_eq!(parsed.format, AgentFileFormat::CopilotAgentMd);
    assert_eq!(parsed.agent.name, "AgentForge Evaluator");
    assert_eq!(parsed.agent.model.model_id, "gpt-4o");
    assert_eq!(parsed.agent.tools.len(), 4);
    assert!(!parsed.agent.system_prompt.is_empty());
}

// ─── Scenario generation (deterministic/adversarial) ──────────────────────

#[tokio::test]
async fn schema_derived_scenarios_generated() {
    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    let agent_id = Uuid::new_v4();

    let scenarios = generate_scenarios(
        &parsed.agent,
        &ScenarioGeneratorConfig {
            total_count: 10,
            agent_id,
            llm_api_key: None, // No LLM — falls back to heuristic
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert!(!scenarios.is_empty(), "Should generate at least 1 scenario");
    assert!(scenarios.len() <= 10);

    // Non-adversarial scenarios should have non-empty user messages;
    // adversarial "empty_input" scenarios are intentionally empty.
    use agentforge_core::ScenarioSource;
    for s in scenarios
        .iter()
        .filter(|s| s.source != ScenarioSource::Adversarial)
    {
        assert!(
            !s.input.user_message.is_empty(),
            "non-adversarial scenario has empty user_message"
        );
    }
}

#[tokio::test]
async fn adversarial_scenarios_include_edge_cases() {
    use agentforge_scenarios::adversarial::generate_adversarial_scenarios;
    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    let scenarios = generate_adversarial_scenarios(&parsed.agent, 10, Uuid::new_v4()).unwrap();
    assert!(
        scenarios.len() >= 5,
        "Expected at least 5 adversarial scenarios"
    );
}

// ─── Runner tests with mocked LLM ─────────────────────────────────────────

#[tokio::test]
async fn runner_completes_with_mocked_llm() {
    let mock_server = MockServer::start().await;

    // Mock OpenAI chat completions endpoint
    let mock_response = serde_json::json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "{\"response\": \"I can help you with that search.\"}"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 50,
            "completion_tokens": 20,
            "total_tokens": 70
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    let agent_id = Uuid::new_v4();

    // Generate a small set of scenarios
    let scenarios = generate_scenarios(
        &parsed.agent,
        &ScenarioGeneratorConfig {
            total_count: 3,
            agent_id,
            llm_api_key: None,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Create a runner pointing at the mock server
    use agentforge_runner::{AgentRunner, OpenAiClient, RunnerConfig};
    let client = std::sync::Arc::new(OpenAiClient::new(
        format!("{}/v1", mock_server.uri()),
        "test-key".to_string(),
        "gpt-4o".to_string(),
    ));

    let runner = AgentRunner::new(
        client,
        RunnerConfig {
            concurrency: 2,
            ..Default::default()
        },
    );
    let run_result = runner.run(&parsed.agent, scenarios.clone(), None).await;
    let traces = run_result.traces;

    assert_eq!(traces.len(), scenarios.len());
}

// ─── Gatekeeper tests ──────────────────────────────────────────────────────

#[test]
fn gatekeeper_first_promotion_approved() {
    use agentforge_core::{DimensionScores, Scorecard};
    use agentforge_gatekeeper::{GateStatus, Gatekeeper, GatekeeperConfig};

    let challenger = Scorecard {
        run_id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        agent_name: "test-agent".to_string(),
        agent_version: "1.0.0".to_string(),
        aggregate_score: 0.88,
        pass_rate: 0.88,
        total_scenarios: 100,
        passed: 88,
        failed: 12,
        errors: 0,
        review_needed: 0,
        dimension_scores: DimensionScores {
            task_completion: 0.90,
            tool_selection: 0.85,
            argument_correctness: 0.88,
            schema_compliance: 0.92,
            instruction_adherence: 0.87,
            path_efficiency: 0.80,
        },
        failure_clusters: vec![],
        duration_seconds: 120,
        total_input_tokens: 10000,
        total_output_tokens: 5000,
    };

    let gk = Gatekeeper::new(GatekeeperConfig::default());
    let decision = gk
        .evaluate(
            Uuid::new_v4(),
            Uuid::new_v4(),
            None,
            &challenger,
            &[],
            &[],
            &[0.88, 0.87, 0.89],
        )
        .unwrap();

    assert!(
        decision.approved,
        "First promotion should be approved automatically"
    );
    assert_eq!(decision.gates[0].status, GateStatus::Waived);
    assert_eq!(decision.gates[1].status, GateStatus::Waived);
}

// ─── Optimizer tests ──────────────────────────────────────────────────────

#[tokio::test]
async fn optimizer_generates_variants_without_llm() {
    use agentforge_core::{DimensionScores, Scorecard};
    use agentforge_optimizer::{Optimizer, OptimizerConfig};

    let parsed = parse_agent_file(SAMPLE_AGENT_YAML).unwrap();
    let scorecard = Scorecard {
        run_id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        agent_name: "test-agent".to_string(),
        agent_version: "1.0.0".to_string(),
        aggregate_score: 0.65,
        pass_rate: 0.65,
        total_scenarios: 100,
        passed: 65,
        failed: 35,
        errors: 0,
        review_needed: 0,
        dimension_scores: DimensionScores {
            task_completion: 0.60,
            tool_selection: 0.70,
            argument_correctness: 0.65,
            schema_compliance: 0.72,
            instruction_adherence: 0.58,
            path_efficiency: 0.80,
        },
        failure_clusters: vec![],
        duration_seconds: 60,
        total_input_tokens: 5000,
        total_output_tokens: 2000,
    };

    let optimizer = Optimizer::new(OptimizerConfig {
        min_variants: 2,
        max_variants: 8,
        llm_api_key: "".to_string(), // No LLM — uses deterministic fallbacks
        ..Default::default()
    });

    let result = optimizer
        .generate_variants(&parsed.agent, &scorecard, &[], "sha123")
        .await
        .unwrap();
    assert!(result.variants.len() >= 2);
    assert!(result.variants.len() <= 8);

    // All variants should have a valid system prompt
    for v in &result.variants {
        assert!(!v.agent.system_prompt.is_empty());
        assert_eq!(v.parent_sha, "sha123");
    }
}

// ─── Scorecard diff test ───────────────────────────────────────────────────

#[test]
fn scorecard_diff_computed_correctly() {
    use agentforge_core::DimensionScores;

    let champ_scores = DimensionScores {
        task_completion: 0.80,
        tool_selection: 0.75,
        argument_correctness: 0.78,
        schema_compliance: 0.85,
        instruction_adherence: 0.82,
        path_efficiency: 0.70,
    };

    let challenger_scores = DimensionScores {
        task_completion: 0.87,
        tool_selection: 0.80,
        argument_correctness: 0.83,
        schema_compliance: 0.88,
        instruction_adherence: 0.84,
        path_efficiency: 0.75,
    };

    use agentforge_core::EvalWeights;
    let weights = EvalWeights::default();

    let champ_agg = champ_scores.weighted_aggregate(&weights);
    let challenger_agg = challenger_scores.weighted_aggregate(&weights);

    assert!(challenger_agg > champ_agg, "Challenger should score higher");
    assert!((challenger_agg - champ_agg) > 0.0);
}
