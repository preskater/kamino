//! OpenAPI document.

use kamino_common::pagination::Page;
use utoipa::OpenApi;

use crate::resources::{
    activity_logs, artifacts, projects, task_comments, task_dependencies, task_reviews, tasks,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Kamino Projects API",
        version = "0.1.0",
        description = "Projects, tasks, dependencies, comments, reviews, artifacts and activity."
    ),
    paths(
        // Projects
        projects::list,
        projects::create,
        projects::fetch,
        projects::update,
        projects::remove,
        // Tasks
        tasks::list,
        tasks::list_for_project,
        tasks::create,
        tasks::create_for_project,
        tasks::fetch,
        tasks::update,
        tasks::remove,
        // Dependencies
        task_dependencies::list_for_task,
        task_dependencies::list_dependents,
        task_dependencies::create_for_task,
        task_dependencies::fetch,
        task_dependencies::update,
        task_dependencies::remove,
        // Comments
        task_comments::list_for_task,
        task_comments::create_for_task,
        task_comments::fetch,
        task_comments::update,
        task_comments::remove,
        // Reviews
        task_reviews::list_for_task,
        task_reviews::create_for_task,
        task_reviews::fetch,
        task_reviews::update,
        task_reviews::remove,
        // Artifacts
        artifacts::list,
        artifacts::list_for_task,
        artifacts::create,
        artifacts::create_for_task,
        artifacts::fetch,
        artifacts::update,
        artifacts::remove,
        // Activity
        activity_logs::list_for_project,
        activity_logs::create_for_project,
        activity_logs::fetch,
        activity_logs::remove,
    ),
    components(schemas(
        // Rows
        projects::Project,
        tasks::Task,
        task_dependencies::TaskDependency,
        task_comments::TaskComment,
        task_reviews::TaskReview,
        artifacts::Artifact,
        activity_logs::ActivityLog,
        // Requests
        projects::CreateProject,
        projects::UpdateProject,
        tasks::CreateTask,
        tasks::UpdateTask,
        task_dependencies::CreateTaskDependency,
        task_dependencies::UpdateTaskDependency,
        task_comments::CreateTaskComment,
        task_comments::UpdateTaskComment,
        task_reviews::CreateTaskReview,
        task_reviews::UpdateTaskReview,
        artifacts::CreateArtifact,
        artifacts::UpdateArtifact,
        activity_logs::CreateActivityLog,
        // Pages
        Page<projects::Project>,
        Page<tasks::Task>,
        Page<task_dependencies::TaskDependency>,
        Page<task_comments::TaskComment>,
        Page<task_reviews::TaskReview>,
        Page<artifacts::Artifact>,
        Page<activity_logs::ActivityLog>,
    )),
    tags(
        (name = "projects", description = "Projects"),
        (name = "tasks", description = "Tasks and their children"),
        (name = "activity", description = "Append-only project activity log")
    )
)]
pub struct ApiDoc;
