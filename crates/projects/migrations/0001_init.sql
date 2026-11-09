-- Kamino projects database.

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE project (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'planning'
        CHECK (status IN ('planning', 'active', 'paused', 'completed', 'archived')),
    start_date DATE,
    target_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER project_set_updated_at
    BEFORE UPDATE ON project
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX project_status_idx ON project (status);

CREATE TABLE task (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    assigned_agent_id UUID,
    specialty TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL,
    initial_prompt TEXT NOT NULL DEFAULT '',
    expected_output TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'in_progress', 'blocked', 'review', 'completed', 'cancelled')),
    priority TEXT NOT NULL DEFAULT 'medium'
        CHECK (priority IN ('low', 'medium', 'high', 'critical')),
    input_context JSONB NOT NULL DEFAULT '{}'::jsonb,
    acceptance_criteria JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX task_project_id_idx ON task (project_id);
CREATE INDEX task_assigned_agent_id_idx ON task (assigned_agent_id);
CREATE INDEX task_status_idx ON task (status);
CREATE INDEX task_priority_idx ON task (priority);

CREATE TABLE task_dependency (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES task (id) ON DELETE CASCADE,
    depends_on_task_id UUID NOT NULL REFERENCES task (id) ON DELETE CASCADE,
    UNIQUE (task_id, depends_on_task_id),
    CHECK (task_id <> depends_on_task_id)
);

CREATE INDEX task_dependency_depends_on_task_id_idx ON task_dependency (depends_on_task_id);

CREATE TABLE task_comment (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES task (id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX task_comment_task_id_idx ON task_comment (task_id);

CREATE TABLE task_review (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES task (id) ON DELETE CASCADE,
    decision TEXT NOT NULL
        CHECK (decision IN ('approved', 'changes_requested', 'rejected')),
    feedback TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'in_review', 'resolved')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX task_review_task_id_idx ON task_review (task_id);

CREATE TABLE artifact (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES task (id) ON DELETE CASCADE,
    agent_run_id UUID,
    artifact_type TEXT NOT NULL DEFAULT 'file',
    name TEXT NOT NULL,
    storage_url TEXT NOT NULL DEFAULT '',
    content TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX artifact_task_id_idx ON artifact (task_id);
CREATE INDEX artifact_agent_run_id_idx ON artifact (agent_run_id);

CREATE TABLE activity_log (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX activity_log_project_id_idx ON activity_log (project_id, created_at DESC);
CREATE INDEX activity_log_entity_idx ON activity_log (entity_type, entity_id);
