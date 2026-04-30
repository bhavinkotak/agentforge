-- Migration 004: Traces
-- Full execution traces per scenario per run

CREATE TYPE trace_status AS ENUM ('pass', 'fail', 'error', 'review_needed');
CREATE TYPE failure_cluster AS ENUM (
    'wrong_tool',
    'hallucinated_argument',
    'looping',
    'premature_stop',
    'schema_violation',
    'constraint_breach',
    'no_failure',
    'unknown'
);

CREATE TABLE traces (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    run_id                  UUID NOT NULL REFERENCES eval_runs (id) ON DELETE CASCADE,
    scenario_id             UUID NOT NULL REFERENCES scenarios (id) ON DELETE CASCADE,
    status                  trace_status NOT NULL DEFAULT 'error',
    steps                   JSONB NOT NULL DEFAULT '[]',    -- full execution trace
    final_output            JSONB,
    scores                  JSONB,                          -- per-dimension scores
    aggregate_score         DOUBLE PRECISION,
    failure_cluster         failure_cluster NOT NULL DEFAULT 'unknown',
    failure_reason          TEXT,
    review_needed           BOOLEAN NOT NULL DEFAULT FALSE,
    llm_calls               INT NOT NULL DEFAULT 0,
    tool_invocations        INT NOT NULL DEFAULT 0,
    input_tokens            INT NOT NULL DEFAULT 0,
    output_tokens           INT NOT NULL DEFAULT 0,
    latency_ms              INT NOT NULL DEFAULT 0,
    retry_count             INT NOT NULL DEFAULT 0,
    seed                    INT NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_traces_run_id ON traces (run_id);
CREATE INDEX idx_traces_scenario_id ON traces (scenario_id);
CREATE INDEX idx_traces_status ON traces (status);
CREATE INDEX idx_traces_failure_cluster ON traces (failure_cluster);
CREATE INDEX idx_traces_review_needed ON traces (review_needed) WHERE review_needed = TRUE;
