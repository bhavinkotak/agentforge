use agentforge_core::{AgentFile, LintError, LintSeverity};

/// Result of validating an agent file.
#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<LintError>,
    pub warnings: Vec<LintError>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn all_issues(&self) -> Vec<&LintError> {
        self.errors.iter().chain(self.warnings.iter()).collect()
    }
}

/// Validate a parsed `AgentFile` and return lint errors and warnings.
pub fn validate_agent_file(agent: &AgentFile) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Required fields
    if agent.name.is_empty() {
        errors.push(lint_error("name", "Agent name must not be empty"));
    }

    if agent.system_prompt.trim().is_empty() {
        errors.push(lint_error("system_prompt", "System prompt must not be empty"));
    }

    if agent.model.model_id.is_empty() {
        errors.push(lint_error("model.model_id", "Model ID must not be empty"));
    }

    // Temperature validation
    if let Some(temp) = agent.model.temperature {
        if !(0.0..=2.0).contains(&temp) {
            errors.push(lint_error(
                "model.temperature",
                &format!("Temperature {temp} is out of valid range [0.0, 2.0]"),
            ));
        }
    }

    // Tool validation
    let tool_names: std::collections::HashSet<&str> = agent.tools.iter()
        .map(|t| t.name.as_str())
        .collect();

    if tool_names.len() != agent.tools.len() {
        errors.push(lint_error("tools", "Duplicate tool names detected"));
    }

    for tool in &agent.tools {
        if tool.name.is_empty() {
            errors.push(lint_error("tools[].name", "Tool name must not be empty"));
        }
        if tool.description.is_empty() {
            warnings.push(lint_warning(
                &format!("tools[{}].description", tool.name),
                "Tool has no description — this reduces scoring accuracy",
            ));
        }
        // Validate tool parameters is a valid JSON Schema object
        if tool.parameters.get("type").is_none() {
            warnings.push(lint_warning(
                &format!("tools[{}].parameters", tool.name),
                "Tool parameters should have a 'type' field",
            ));
        }
    }

    // Output schema validation
    if let Some(schema) = &agent.output_schema {
        if schema.get("type").is_none() && schema.get("$ref").is_none() {
            warnings.push(lint_warning(
                "output_schema",
                "Output schema should specify a 'type' field",
            ));
        }
    } else {
        warnings.push(lint_warning(
            "output_schema",
            "No output schema defined — output schema compliance scoring will be skipped",
        ));
    }

    // Eval hints validation
    if let Some(hints) = &agent.eval_hints {
        if let Some(threshold) = hints.pass_threshold {
            if !(0.0..=1.0).contains(&threshold) {
                errors.push(lint_error(
                    "eval_hints.pass_threshold",
                    &format!("pass_threshold {threshold} must be between 0.0 and 1.0"),
                ));
            }
        }
        if let Some(count) = hints.scenario_count {
            if count == 0 {
                errors.push(lint_error(
                    "eval_hints.scenario_count",
                    "scenario_count must be > 0",
                ));
            }
            if count > 2000 {
                warnings.push(lint_warning(
                    "eval_hints.scenario_count",
                    &format!("scenario_count {count} exceeds recommended max of 2000"),
                ));
            }
        }

        // Check that critical_tools reference actual defined tools
        for critical_tool in &hints.critical_tools {
            if !tool_names.contains(critical_tool.as_str()) {
                warnings.push(lint_warning(
                    "eval_hints.critical_tools",
                    &format!("Critical tool '{critical_tool}' is not defined in tools[]"),
                ));
            }
        }
    }

    // Constraints validation
    if agent.constraints.is_empty() {
        warnings.push(lint_warning(
            "constraints",
            "No constraints defined — instruction adherence scoring will be limited",
        ));
    }

    ValidationResult { errors, warnings }
}

fn lint_error(field: &str, message: &str) -> LintError {
    LintError {
        field: field.to_string(),
        message: message.to_string(),
        severity: LintSeverity::Error,
    }
}

fn lint_warning(field: &str, message: &str) -> LintError {
    LintError {
        field: field.to_string(),
        message: message.to_string(),
        severity: LintSeverity::Warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentforge_core::{AgentFile, EvalHints, ModelConfig, ModelProvider, ToolDefinition};

    fn make_valid_agent() -> AgentFile {
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
            system_prompt: "You are a helpful assistant.".to_string(),
            tools: vec![ToolDefinition {
                name: "search".to_string(),
                description: "Search the web".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            }],
            output_schema: Some(serde_json::json!({"type": "object"})),
            constraints: vec!["Always be polite.".to_string()],
            eval_hints: Some(EvalHints {
                domain: Some("general".to_string()),
                typical_turns: Some(3),
                critical_tools: vec!["search".to_string()],
                pass_threshold: Some(0.85),
                scenario_count: Some(100),
            }),
            metadata: None,
        }
    }

    #[test]
    fn valid_agent_passes() {
        let agent = make_valid_agent();
        let result = validate_agent_file(&agent);
        assert!(result.is_valid(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn empty_name_is_error() {
        let mut agent = make_valid_agent();
        agent.name = "".to_string();
        let result = validate_agent_file(&agent);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.field == "name"));
    }

    #[test]
    fn invalid_temperature_is_error() {
        let mut agent = make_valid_agent();
        agent.model.temperature = Some(3.0);
        let result = validate_agent_file(&agent);
        assert!(!result.is_valid());
    }

    #[test]
    fn undefined_critical_tool_is_warning() {
        let mut agent = make_valid_agent();
        if let Some(hints) = agent.eval_hints.as_mut() {
            hints.critical_tools = vec!["nonexistent_tool".to_string()];
        }
        let result = validate_agent_file(&agent);
        assert!(result.is_valid()); // It's a warning, not an error
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn missing_output_schema_is_warning() {
        let mut agent = make_valid_agent();
        agent.output_schema = None;
        let result = validate_agent_file(&agent);
        assert!(result.is_valid()); // Warning, not error
        assert!(result.warnings.iter().any(|w| w.field == "output_schema"));
    }

    #[test]
    fn duplicate_tool_names_is_error() {
        let mut agent = make_valid_agent();
        agent.tools.push(agent.tools[0].clone());
        let result = validate_agent_file(&agent);
        assert!(!result.is_valid());
    }
}
