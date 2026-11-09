-- Kamino agents database.

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE agent_template (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    specialty TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    default_instructions TEXT NOT NULL DEFAULT '',
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    allowed_tools JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_skill_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_model TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_template_specialty_idx ON agent_template (specialty);

CREATE TABLE agent (
    id UUID PRIMARY KEY,
    agent_template_id UUID NOT NULL REFERENCES agent_template (id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    specialty TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'inactive', 'archived')),
    configuration JSONB NOT NULL DEFAULT '{}'::jsonb,
    tool_permissions JSONB NOT NULL DEFAULT '{}'::jsonb,
    model TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_agent_template_id_idx ON agent (agent_template_id);
CREATE INDEX agent_status_idx ON agent (status);
CREATE INDEX agent_specialty_idx ON agent (specialty);

CREATE TABLE skill (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    version TEXT NOT NULL DEFAULT '0.1.0',
    source_type TEXT NOT NULL DEFAULT 'local'
        CHECK (source_type IN ('local', 'git', 'registry', 'url')),
    source_url TEXT,
    storage_path TEXT,
    skill_md_content TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    required_permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER skill_set_updated_at
    BEFORE UPDATE ON skill
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE skill_version (
    id UUID PRIMARY KEY,
    skill_id UUID NOT NULL REFERENCES skill (id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    changelog TEXT,
    storage_path TEXT,
    skill_md_content TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_current BOOLEAN NOT NULL DEFAULT false,
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (skill_id, version)
);

CREATE UNIQUE INDEX skill_version_current_idx
    ON skill_version (skill_id)
    WHERE is_current;

CREATE TABLE skill_file (
    id UUID PRIMARY KEY,
    skill_version_id UUID NOT NULL REFERENCES skill_version (id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    file_type TEXT NOT NULL DEFAULT 'text',
    storage_url TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL DEFAULT 0 CHECK (file_size_bytes >= 0),
    checksum TEXT NOT NULL DEFAULT '',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX skill_file_skill_version_id_idx ON skill_file (skill_version_id);

CREATE TABLE agent_skill (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agent (id) ON DELETE CASCADE,
    skill_id UUID NOT NULL REFERENCES skill (id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'enabled'
        CHECK (status IN ('enabled', 'disabled')),
    configuration JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agent_id, skill_id)
);

CREATE INDEX agent_skill_skill_id_idx ON agent_skill (skill_id);

CREATE TABLE agent_run (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agent (id) ON DELETE CASCADE,
    task_id UUID,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    model TEXT NOT NULL DEFAULT '',
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK (output_tokens >= 0),
    cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE INDEX agent_run_agent_id_idx ON agent_run (agent_id);
CREATE INDEX agent_run_task_id_idx ON agent_run (task_id);
CREATE INDEX agent_run_status_idx ON agent_run (status);

CREATE TABLE task_skill_usage (
    id UUID PRIMARY KEY,
    agent_run_id UUID NOT NULL REFERENCES agent_run (id) ON DELETE CASCADE,
    skill_id UUID NOT NULL REFERENCES skill (id) ON DELETE CASCADE,
    activation_reason TEXT NOT NULL DEFAULT '',
    loaded_files JSONB NOT NULL DEFAULT '[]'::jsonb,
    activated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX task_skill_usage_agent_run_id_idx ON task_skill_usage (agent_run_id);
CREATE INDEX task_skill_usage_skill_id_idx ON task_skill_usage (skill_id);

CREATE TABLE agent_message (
    id UUID PRIMARY KEY,
    agent_run_id UUID NOT NULL REFERENCES agent_run (id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL DEFAULT '',
    tool_calls JSONB NOT NULL DEFAULT '[]'::jsonb,
    tool_results JSONB NOT NULL DEFAULT '[]'::jsonb,
    token_count INTEGER NOT NULL DEFAULT 0 CHECK (token_count >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_message_agent_run_id_idx ON agent_message (agent_run_id);

CREATE TABLE agent_tool_call (
    id UUID PRIMARY KEY,
    agent_run_id UUID NOT NULL REFERENCES agent_run (id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    arguments JSONB NOT NULL DEFAULT '{}'::jsonb,
    result JSONB,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'success', 'error')),
    called_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_tool_call_agent_run_id_idx ON agent_tool_call (agent_run_id);

CREATE TABLE agent_artifact (
    id UUID PRIMARY KEY,
    agent_run_id UUID NOT NULL REFERENCES agent_run (id) ON DELETE CASCADE,
    task_id UUID,
    artifact_type TEXT NOT NULL DEFAULT 'file',
    name TEXT NOT NULL,
    storage_url TEXT NOT NULL DEFAULT '',
    content TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_artifact_agent_run_id_idx ON agent_artifact (agent_run_id);
CREATE INDEX agent_artifact_task_id_idx ON agent_artifact (task_id);

CREATE TABLE agent_metric (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agent (id) ON DELETE CASCADE,
    metric_date DATE NOT NULL,
    runs_count INTEGER NOT NULL DEFAULT 0 CHECK (runs_count >= 0),
    successful_runs INTEGER NOT NULL DEFAULT 0 CHECK (successful_runs >= 0),
    failed_runs INTEGER NOT NULL DEFAULT 0 CHECK (failed_runs >= 0),
    total_input_tokens INTEGER NOT NULL DEFAULT 0 CHECK (total_input_tokens >= 0),
    total_output_tokens INTEGER NOT NULL DEFAULT 0 CHECK (total_output_tokens >= 0),
    total_cost_usd NUMERIC(12, 6) NOT NULL DEFAULT 0,
    UNIQUE (agent_id, metric_date)
);
