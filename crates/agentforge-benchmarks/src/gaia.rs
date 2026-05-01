use agentforge_core::{BenchmarkSuite, BenchmarkTask};
use std::io::BufRead;

/// Parse GAIA benchmark tasks from a JSONL file.
///
/// GAIA task format:
/// ```json
/// {"task_id": "...", "Question": "...", "Final answer": "...", "Level": 1}
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
                    tracing::warn!(line = i, error = %e, "Skipping malformed GAIA task");
                    None
                }
            }
        })
        .collect()
}

fn parse_task(v: &serde_json::Value, index: usize) -> BenchmarkTask {
    let id = v
        .get("task_id")
        .and_then(|x| x.as_str())
        .unwrap_or(&format!("gaia-{index}"))
        .to_string();

    let question = v
        .get("Question")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let expected_answer = v
        .get("Final answer")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    let difficulty_level = v.get("Level").and_then(|x| x.as_u64()).map(|l| l as u8);

    BenchmarkTask {
        id,
        suite: BenchmarkSuite::Gaia,
        difficulty_level,
        question,
        expected_answer,
        context_files: vec![],
    }
}

/// Load GAIA tasks from a `BufRead` source (file or stdin).
pub fn load_from_reader<R: BufRead>(reader: R) -> Vec<BenchmarkTask> {
    let content: String = reader
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<_>>()
        .join("\n");
    load_from_jsonl(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_gaia_task() {
        let jsonl = r#"{"task_id":"g1","Question":"What is 2+2?","Final answer":"4","Level":1}"#;
        let tasks = load_from_jsonl(jsonl);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "g1");
        assert_eq!(tasks[0].expected_answer.as_deref(), Some("4"));
        assert_eq!(tasks[0].difficulty_level, Some(1));
    }

    #[test]
    fn skip_malformed_lines() {
        let jsonl = "not-json\n{\"task_id\":\"g2\",\"Question\":\"Q\",\"Final answer\":\"A\"}";
        let tasks = load_from_jsonl(jsonl);
        assert_eq!(tasks.len(), 1);
    }
}
