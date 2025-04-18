use std::str::FromStr;
#[allow(unused_imports)]
use crate::security::permission::PermissionResult;
use common::{ProjectId, ProjectSlug, ProjectSlugStr};
#[allow(unused_imports)]
use leptos::logging::log;
use leptos::prelude::ServerFnError;
use leptos::server;
use serde::{Deserialize, Serialize};
use crate::models::Project;

#[server]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    use crate::security::utils::ssr::get_auth_session_user_id;


    let pool = crate::ssr::pool()?;
    let auth = crate::ssr::auth(false)?;
    let projects = sqlx::query_as!(Project,
        "SELECT * FROM projects WHERE id IN (SELECT project_id FROM permissions WHERE user_id = $1)",
        get_auth_session_user_id(&auth).unwrap()
    )
        .fetch_all(&pool)
        .await.
        map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(projects)
}

#[server]
pub async fn get_project(project_slug:ProjectSlugStr) -> Result<Project, ServerFnError> {
    
    use common::permission::Permission;
    
    let project_id = ProjectSlug::from_str(project_slug.as_str()).map_err(|e| {
        leptos_axum::redirect("/user/projects");
        ServerFnError::new(e.to_string())
    })?.id;
    
    let pool = crate::ssr::pool()?;
    let auth = crate::ssr::auth(false)?;
    match crate::security::permission::ssr::ensure_permission(&auth, project_id, Permission::Read).await? {
        PermissionResult::Allow => {
            let project = sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", project_id)
                .fetch_one(&pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(project)
        }
        PermissionResult::Redirect(path) => {
            leptos_axum::redirect(path.as_str());
            Err(ServerFnError::new("Permission denied"))
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::security::permission::ssr::SqlPermissionType;
    use common::server_action::user_action::UserAction;
    use common::{Slug, UserSlug};
    use leptos::prelude::ServerFnError;
    use crate::api::ssr::request_server_action;
    use crate::models::Project;

    pub async fn create_project(user_slug: UserSlug, name: String) -> Result<Project, ServerFnError> {
        let pool = crate::ssr::pool()?;
        let project = sqlx::query!("INSERT INTO projects (name) VALUES ($1) returning id", name)
            .fetch_one(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        sqlx::query!(
            "INSERT INTO permissions (user_id, project_id, permission) VALUES ($1, $2, $3)",
            user_slug.id,
            project.id,
            SqlPermissionType::Owner as SqlPermissionType
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        request_server_action(
            UserAction::AddProject {
                user_slug,
                project_slug:Slug::new(project.id, name.clone()),
            }
            .into(),
        )
        .await?;
        Ok(Project{
            id: project.id,
            name,
        })
    }
}
