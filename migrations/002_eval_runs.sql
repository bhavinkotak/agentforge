-- Migration 002: Eval Runs
-- Tracks each evaluation run: which agent version, which scenario set, scores

CREATE TYPE eval_run_status AS ENUM ('pending', 'running', 'complete', 'error', 'cancelled');

CREATE TABLE eval_runs (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id            UUID NOT NULL REFERENCES agent_versions (id) ON DELETE CASCADE,
    scenario_set_id     UUID,   -- optional: links to a named scenario set
    status              eval_run_status NOT NULL DEFAULT 'pending',
    scenario_count      INT NOT NULL DEFAULT 0,
    completed_count     INT NOT NULL DEFAULT 0,
    error_count         INT NOT NULL DEFAULT 0,
    aggregate_score     DOUBLE PRECISION,
    pass_rate           DOUBLE PRECISION,
    task_completion     DOUBLE PRECISION,
    tool_selection      DOUBLE PRECISION,
    argument_correctness DOUBLE PRECISION,
    path_efficiency     DOUBLE PRECISION,
    schema_compliance   DOUBLE PRECISION,
    instruction_adherence DOUBLE PRECISION,
    failure_clusters    JSONB,      -- summary of failure categories
    seed                INT NOT NULL DEFAULT 0,
    concurrency         INT NOT NULL DEFAULT 10,
    error_message       TEXT,
    started_at          TIMESTAMPTZ,
    completed_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_eval_runs_agent_id ON eval_runs (agent_id);
CREATE INDEX idx_eval_runs_status ON eval_runs (status);
CREATE INDEX idx_eval_runs_created_at ON eval_runs (created_at DESC);

CREATE TRIGGER update_eval_runs_updated_at
    BEFORE UPDATE ON eval_runs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
