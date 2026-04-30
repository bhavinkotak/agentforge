-- 007: benchmark runs and per-task results
CREATE TABLE IF NOT EXISTS benchmark_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id        UUID NOT NULL REFERENCES agent_versions(id),
    suite           TEXT NOT NULL
                        CHECK (suite IN ('gaia', 'agentbench', 'webarena')),
    total_tasks     INTEGER NOT NULL DEFAULT 0,
    correct         INTEGER NOT NULL DEFAULT 0,
    accuracy        DOUBLE PRECISION,
    percentile_rank DOUBLE PRECISION,
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'running', 'complete', 'error')),
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS benchmark_results (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    benchmark_run_id UUID NOT NULL REFERENCES benchmark_runs(id) ON DELETE CASCADE,
    task_id         TEXT NOT NULL,
    suite           TEXT NOT NULL,
    difficulty_level SMALLINT,
    agent_answer    TEXT,
    expected_answer TEXT,
    correct         BOOLEAN NOT NULL DEFAULT FALSE,
    score           DOUBLE PRECISION NOT NULL DEFAULT 0,
    latency_ms      BIGINT,
    token_cost_usd  DOUBLE PRECISION,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_benchmark_runs_agent_id ON benchmark_runs(agent_id);
CREATE INDEX idx_benchmark_runs_suite    ON benchmark_runs(suite);
CREATE INDEX idx_benchmark_results_run   ON benchmark_results(benchmark_run_id);
