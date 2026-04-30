-- 006: fine-tune export jobs
CREATE TABLE IF NOT EXISTS finetune_exports (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id          UUID NOT NULL REFERENCES eval_runs(id),
    format          TEXT NOT NULL
                        CHECK (format IN ('openai', 'anthropic', 'huggingface')),
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'running', 'complete', 'error')),
    row_count       INTEGER,
    file_path       TEXT,
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_finetune_exports_run_id ON finetune_exports(run_id);
CREATE INDEX idx_finetune_exports_status ON finetune_exports(status);
