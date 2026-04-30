use agentforge_core::{BenchmarkSuite, BenchmarkTask};

/// Parse WebArena benchmark tasks from a JSONL file.
///
/// WebArena task format:
/// ```json
/// {"task_id": 0, "intent": "...", "eval": {"reference_answers": {"must_include": ["..."]}}}
/// ```
pub fn load_from_jsonl(jsonl: &str) -> Vec<BenchmarkTask> {
    jsonl
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(v) => Some(parse_task(&v, i)),
                Err(e) => {
                    tracing::warn!(line = i, error = %e, "Skipping malformed WebArena task");
                    None
                }
            }
        })
        .collect()
}

fn parse_task(v: &serde_json::Value, index: usize) -> BenchmarkTask {
    let id = v
        .get("task_id")
        .and_then(|x| x.as_u64())
        .map(|n| format!("webarena-{n}"))
        .unwrap_or_else(|| format!("webarena-{index}"));

    let question = v
        .get("intent")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    // Extract expected answer from eval.reference_answers.must_include[0]
    let expected_answer = v
        .pointer("/eval/reference_answers/must_include/0")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    // Context files from start_url if present
    let context_files = v
        .get("start_url")
        .and_then(|x| x.as_str())
        .map(|url| vec![url.to_string()])
        .unwrap_or_default();

    BenchmarkTask {
        id,
        suite: BenchmarkSuite::WebArena,
        difficulty_level: None,
        question,
        expected_answer,
        context_files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_webarena_task() {
        let jsonl = r#"{"task_id":1,"intent":"Find the price of item X","eval":{"reference_answers":{"must_include":["$29.99"]}}}"#;
        let tasks = load_from_jsonl(jsonl);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].expected_answer.as_deref(), Some("$29.99"));
    }
}
