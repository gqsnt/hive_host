#[allow(unused_imports)]
use crate::permission::PermissionResult;
use common::permission::Permission;
use common::{ProjectId, ProjectSlug, ProjectSlugStr, UserId};
#[allow(unused_imports)]
use leptos::logging::log;
use leptos::prelude::ServerFnError;
use leptos::server;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
}

impl Project {
    pub fn get_slug(&self) -> ProjectSlug {
        ProjectSlug::new(self.id, self.name.clone())
    }
}

#[server]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    let pool = crate::ssr::pool()?;
    let auth = crate::ssr::auth(false)?;
    let projects = sqlx::query_as!(Project,
        "SELECT * FROM projects WHERE id IN (SELECT project_id FROM permissions WHERE user_id = $1)",
        crate::auth::ssr::get_auth_session_user_id(&auth).unwrap()
    )
        .fetch_all(&pool)
        .await.
        map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(projects)
}

#[server]
pub async fn get_project(project_id:ProjectId) -> Result<Project, ServerFnError> {
    let pool = crate::ssr::pool()?;
    let auth = crate::ssr::auth(false)?;
    log!("Session Id: {}", auth.session.get_session_id());
    match crate::permission::ssr::ensure_permission(&auth,project_id , Permission::Read).await? {
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
    use crate::permission::ssr::{request_server_action, SqlPermissionType};
    use common::server_action::user_action::UserAction;
    use common::{Slug, UserId, UserSlug};
    use leptos::prelude::ServerFnError;

    pub async fn create_project(user_slug: UserSlug, name: String) -> Result<(), ServerFnError> {
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
                project_slug:Slug::new(project.id, name),
            }
            .into(),
        )
        .await?;
        Ok(())
    }
}
