-- 008: add cost tracking columns to traces
ALTER TABLE traces
    ADD COLUMN IF NOT EXISTS cost_usd DOUBLE PRECISION;

-- Materialised convenience view: cost per run
CREATE OR REPLACE VIEW run_cost_summary AS
SELECT
    t.run_id,
    SUM(t.input_tokens)   AS total_input_tokens,
    SUM(t.output_tokens)  AS total_output_tokens,
    SUM(t.cost_usd)       AS total_cost_usd,
    COUNT(*)              AS trace_count
FROM traces t
GROUP BY t.run_id;
