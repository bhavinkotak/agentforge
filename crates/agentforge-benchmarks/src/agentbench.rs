use agentforge_core::{BenchmarkSuite, BenchmarkTask};

/// Parse AgentBench benchmark tasks from a JSONL file.
///
/// AgentBench task format:
/// ```json
/// {"id": "...", "task": "...", "answer": "...", "env": "os"}
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
                    tracing::warn!(line = i, error = %e, "Skipping malformed AgentBench task");
                    None
                }
            }
        })
        .collect()
}

fn parse_task(v: &serde_json::Value, index: usize) -> BenchmarkTask {
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or(&format!("agentbench-{index}"))
        .to_string();

    let question = v
        .get("task")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let expected_answer = v
        .get("answer")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    BenchmarkTask {
        id,
        suite: BenchmarkSuite::AgentBench,
        difficulty_level: None,
        question,
        expected_answer,
        context_files: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_agentbench_task() {
        let jsonl = r#"{"id":"ab1","task":"List files in /tmp","answer":"file1.txt","env":"os"}"#;
        let tasks = load_from_jsonl(jsonl);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "ab1");
    }
}
