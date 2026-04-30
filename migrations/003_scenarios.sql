-- Migration 003: Scenarios
-- Stores generated test scenarios, versioned and reusable across agent versions

CREATE TYPE difficulty_tier AS ENUM ('easy', 'medium', 'hard', 'edge');
CREATE TYPE scenario_source AS ENUM ('schema_derived', 'adversarial', 'domain_seeded', 'manual');

CREATE TABLE scenarios (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id        UUID NOT NULL REFERENCES agent_versions (id) ON DELETE CASCADE,
    input           JSONB NOT NULL,
    expected        JSONB NOT NULL,     -- expected_tool_calls, expected_output_schema, pass_criteria
    difficulty      difficulty_tier NOT NULL DEFAULT 'medium',
    domain          TEXT,
    source          scenario_source NOT NULL DEFAULT 'schema_derived',
    tags            TEXT[] NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scenarios_agent_id ON scenarios (agent_id);
CREATE INDEX idx_scenarios_difficulty ON scenarios (difficulty);
CREATE INDEX idx_scenarios_source ON scenarios (source);
