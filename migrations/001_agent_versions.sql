-- Migration 001: Agent Versions
-- Stores versioned agent file artifacts with SHA-based content addressing

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE agent_versions (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name            TEXT NOT NULL,
    version         TEXT NOT NULL,
    sha             TEXT NOT NULL UNIQUE,
    file_content    JSONB NOT NULL,
    raw_content     TEXT NOT NULL,
    format          TEXT NOT NULL,  -- native_yaml | openai_json | anthropic_json | langchain_yaml | crewai_yaml
    promoted        BOOLEAN NOT NULL DEFAULT FALSE,
    is_champion     BOOLEAN NOT NULL DEFAULT FALSE,
    changelog       TEXT,           -- diff summary when promoted
    parent_sha      TEXT,           -- SHA of the agent version this was derived from (optimizer lineage)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_versions_name ON agent_versions (name);
CREATE INDEX idx_agent_versions_sha ON agent_versions (sha);
CREATE INDEX idx_agent_versions_champion ON agent_versions (name, is_champion) WHERE is_champion = TRUE;

-- Trigger to update updated_at automatically
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_agent_versions_updated_at
    BEFORE UPDATE ON agent_versions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
