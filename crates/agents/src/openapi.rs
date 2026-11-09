//! OpenAPI document.

use utoipa::OpenApi;

use crate::resources::{
    agent_artifacts, agent_messages, agent_metrics, agent_runs, agent_skills, agent_templates,
    agent_tool_calls, agents, skill_files, skill_versions, skills, task_skill_usage,
};

use kamino_query::Page;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Kamino Agents API",
        version = "0.1.0",
        description = "Agents, templates, skills, runs, messages, tool calls, artifacts and metrics."
    ),
    paths(
        // Agent templates
        agent_templates::list,
        agent_templates::create,
        agent_templates::fetch,
        agent_templates::update,
        agent_templates::remove,
        // Agents
        agents::list,
        agents::list_for_template,
        agents::create,
        agents::create_for_template,
        agents::fetch,
        agents::update,
        agents::remove,
        // Skills
        skills::list,
        skills::create,
        skills::fetch,
        skills::update,
        skills::remove,
        // Skill versions
        skill_versions::list_for_skill,
        skill_versions::create_for_skill,
        skill_versions::fetch,
        skill_versions::update,
        skill_versions::remove,
        skill_versions::publish,
        // Skill files
        skill_files::list_for_version,
        skill_files::create_for_version,
        skill_files::fetch,
        skill_files::update,
        skill_files::remove,
        // Agent skills
        agent_skills::list_for_agent,
        agent_skills::create_for_agent,
        agent_skills::fetch,
        agent_skills::update,
        agent_skills::remove,
        // Runs
        agent_runs::list,
        agent_runs::create,
        agent_runs::fetch,
        agent_runs::update,
        agent_runs::remove,
        // Skill usages
        task_skill_usage::list_for_run,
        task_skill_usage::create_for_run,
        task_skill_usage::fetch,
        task_skill_usage::update,
        task_skill_usage::remove,
        // Messages
        agent_messages::list_for_run,
        agent_messages::create_for_run,
        agent_messages::fetch,
        agent_messages::update,
        agent_messages::remove,
        // Tool calls
        agent_tool_calls::list_for_run,
        agent_tool_calls::create_for_run,
        agent_tool_calls::fetch,
        agent_tool_calls::update,
        agent_tool_calls::remove,
        // Artifacts
        agent_artifacts::list,
        agent_artifacts::create,
        agent_artifacts::fetch,
        agent_artifacts::update,
        agent_artifacts::remove,
        // Metrics
        agent_metrics::list_for_agent,
        agent_metrics::create_for_agent,
        agent_metrics::fetch,
        agent_metrics::update,
        agent_metrics::remove,
    ),
    components(schemas(
        // Rows
        agent_templates::AgentTemplate,
        agents::Agent,
        skills::Skill,
        skill_versions::SkillVersion,
        skill_files::SkillFile,
        agent_skills::AgentSkill,
        agent_runs::AgentRun,
        task_skill_usage::TaskSkillUsage,
        agent_messages::AgentMessage,
        agent_tool_calls::AgentToolCall,
        agent_artifacts::AgentArtifact,
        agent_metrics::AgentMetric,
        // Requests
        agent_templates::CreateAgentTemplate,
        agent_templates::UpdateAgentTemplate,
        agents::CreateAgent,
        agents::UpdateAgent,
        skills::CreateSkill,
        skills::UpdateSkill,
        skill_versions::CreateSkillVersion,
        skill_versions::UpdateSkillVersion,
        skill_files::CreateSkillFile,
        skill_files::UpdateSkillFile,
        agent_skills::CreateAgentSkill,
        agent_skills::UpdateAgentSkill,
        agent_runs::CreateAgentRun,
        agent_runs::UpdateAgentRun,
        task_skill_usage::CreateTaskSkillUsage,
        task_skill_usage::UpdateTaskSkillUsage,
        agent_messages::CreateAgentMessage,
        agent_messages::UpdateAgentMessage,
        agent_tool_calls::CreateAgentToolCall,
        agent_tool_calls::UpdateAgentToolCall,
        agent_artifacts::CreateAgentArtifact,
        agent_artifacts::UpdateAgentArtifact,
        agent_metrics::CreateAgentMetric,
        agent_metrics::UpdateAgentMetric,
        // Pages
        Page<agent_templates::AgentTemplate>,
        Page<agents::Agent>,
        Page<skills::Skill>,
        Page<skill_versions::SkillVersion>,
        Page<skill_files::SkillFile>,
        Page<agent_skills::AgentSkill>,
        Page<agent_runs::AgentRun>,
        Page<task_skill_usage::TaskSkillUsage>,
        Page<agent_messages::AgentMessage>,
        Page<agent_tool_calls::AgentToolCall>,
        Page<agent_artifacts::AgentArtifact>,
        Page<agent_metrics::AgentMetric>,
    )),
    tags(
        (name = "agent-templates", description = "Reusable agent blueprints"),
        (name = "agents", description = "Configured agent instances"),
        (name = "skills", description = "Installable skill packages"),
        (name = "runs", description = "Agent executions"),
        (name = "metrics", description = "Aggregated agent usage")
    )
)]
pub struct ApiDoc;
