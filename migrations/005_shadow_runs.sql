-- 005: shadow runs table for online eval (shadow mode)
CREATE TABLE IF NOT EXISTS shadow_runs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    champion_agent_id   UUID NOT NULL REFERENCES agent_versions(id),
    candidate_agent_id  UUID NOT NULL REFERENCES agent_versions(id),
    traffic_percent     SMALLINT NOT NULL DEFAULT 10
                            CHECK (traffic_percent BETWEEN 1 AND 100),
    status              TEXT NOT NULL DEFAULT 'pending'
                            CHECK (status IN ('pending', 'running', 'complete', 'error')),
    comparison_result   JSONB,
    error_message       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at          TIMESTAMPTZ,
    completed_at        TIMESTAMPTZ
);

CREATE INDEX idx_shadow_runs_champion  ON shadow_runs(champion_agent_id);
CREATE INDEX idx_shadow_runs_candidate ON shadow_runs(candidate_agent_id);
CREATE INDEX idx_shadow_runs_status    ON shadow_runs(status);
