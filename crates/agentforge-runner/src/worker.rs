// Worker module: re-exports and worker utilities
// Individual workers are managed by AgentRunner via tokio::spawn
// This module provides worker-level utilities

use agentforge_core::Trace;

/// Summarize a batch of traces for progress reporting.
pub fn summarize_batch(traces: &[Trace]) -> BatchSummary {
    let total = traces.len() as u32;
    let passed = traces
        .iter()
        .filter(|t| t.status == agentforge_core::TraceStatus::Pass)
        .count() as u32;
    let failed = traces
        .iter()
        .filter(|t| t.status == agentforge_core::TraceStatus::Fail)
        .count() as u32;
    let errors = traces
        .iter()
        .filter(|t| t.status == agentforge_core::TraceStatus::Error)
        .count() as u32;
    let review = traces.iter().filter(|t| t.review_needed).count() as u32;
    let total_tokens: u64 = traces
        .iter()
        .map(|t| (t.input_tokens + t.output_tokens) as u64)
        .sum();

    BatchSummary {
        total,
        passed,
        failed,
        errors,
        review,
        total_tokens,
    }
}

#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub errors: u32,
    pub review: u32,
    pub total_tokens: u64,
}

impl BatchSummary {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.passed as f64) / (self.total as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_rate_zero_when_empty() {
        let summary = BatchSummary {
            total: 0,
            passed: 0,
            failed: 0,
            errors: 0,
            review: 0,
            total_tokens: 0,
        };
        assert_eq!(summary.pass_rate(), 0.0);
    }

    #[test]
    fn pass_rate_correct() {
        let summary = BatchSummary {
            total: 10,
            passed: 7,
            failed: 2,
            errors: 1,
            review: 0,
            total_tokens: 1000,
        };
        assert!((summary.pass_rate() - 0.7).abs() < 1e-9);
    }
}
