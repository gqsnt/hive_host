use std::str::FromStr;
use leptos::prelude::ServerFnError;
use leptos::server;
use common::{ProjectSlug, ProjectSlugStr};
use common::server_project_action::{IsProjectServerAction, ServerProjectAction, ServerProjectActionResponse};

pub fn token_url(token: String) -> String {
    format!("http://127.0.1:3002/token/{}", token)
}



#[server]
pub async fn request_server_project_action_front(
    project_slug: ProjectSlugStr,
    action: ServerProjectAction,
) -> Result<ServerProjectActionResponse, ServerFnError> {
    use common::server_project_action::{IsProjectServerAction, ServerProjectActionRequest};
    use secrecy::ExposeSecret;
    use crate::api::ssr::{request_server_project_action, request_server_project_action_token};
    use crate::security::permission::ssr::ensure_permission;
    
    let project_id = ProjectSlug::from_str(project_slug.as_str()).map_err(|e| {
        leptos_axum::redirect("/user/projects");
        ServerFnError::new(e.to_string())
    })?.id;



    let auth = crate::ssr::auth(false)?;
    match ensure_permission(&auth, project_id, action.permission()).await? {
        PermissionResult::Allow => {
            let pool = crate::ssr::pool()?;
            let project = sqlx::query!(
                r#"SELECT id, name FROM projects WHERE id = $1"#,
                project_id
            ).fetch_one(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;
            let project_slug = ProjectSlug::new(project.id, project.name);
            if action.with_token() {
                request_server_project_action_token(project_slug, action).await
            } else {
                request_server_project_action(project_slug, action).await
            }
        }
        PermissionResult::Redirect(to_path) => {
            leptos_axum::redirect(&to_path);
            Err(ServerFnError::new("Permission denied"))
        }
    }
}


pub enum PermissionResult {
    Allow,
    Redirect(String),
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use std::str::FromStr;
    use crate::security::permission::PermissionResult;
    use crate::security::ssr::AppAuthSession;
    use crate::security::utils::ssr::get_auth_session_user_id;
    use crate::ssr::{permissions, pool, Permissions};
    use common::permission::Permission;
    use common::{ProjectId, ProjectSlug, ProjectSlugStr, UserId};
    use leptos::logging::log;
    use leptos::prelude::ServerFnError;
    use sqlx::PgPool;
    use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct SqlPermission {
        pub(crate) permission: Permission,
    }







    pub async fn request_project_permission(
        permissions: Permissions,
        user_id: UserId,
        project_id: ProjectId,
    ) -> Result<Option<Permission>, ServerFnError> {
        let pool = pool()?;
        let permission = sqlx::query_as!(
    SqlPermission,
    r#"SELECT permission as "permission: Permission" FROM permissions WHERE user_id = $1 AND project_id = $2"#,
    user_id,
    project_id
)
            .fetch_optional(&pool)
            .await?;
        if let Some(permission) = permission {
            let permission = permission.permission.into();
            permissions.insert((user_id, project_id), permission).await;
            Ok(Some(permission))
        } else {
            Ok(None)
        }
    }


    pub async fn handle_project_permission_request(
        project_slug:ProjectSlugStr,
        permission:Permission
    ) ->  Result<(AppAuthSession,PgPool, ProjectSlug), ServerFnError>{
        let project_id = ProjectSlug::from_str(project_slug.as_str()).map_err(|e| {
            leptos_axum::redirect("/user/projects");
            ServerFnError::new(e.to_string())
        })?.id;
        let auth = crate::ssr::auth(false)?;
        match ensure_permission(&auth, project_id, permission).await? {
            PermissionResult::Allow => {
                let pool = crate::ssr::pool()?;
                let project = sqlx::query!(
                r#"SELECT id, name FROM projects WHERE id = $1"#,
                project_id
            ).fetch_one(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;
                let project_slug = ProjectSlug::new(project.id, project.name);
                Ok((auth, pool, project_slug))
            }
            PermissionResult::Redirect(to_path) => {
                leptos_axum::redirect(&to_path);
                Err(ServerFnError::new("Permission denied"))
            }
        }
    }


    pub async fn ensure_permission(
        auth_session: &AppAuthSession,
        project_id: ProjectId,
        permission_type: Permission,
    ) -> Result<PermissionResult, ServerFnError> {
        let permissions = permissions()?;
        if let Some(user_id) = get_auth_session_user_id(auth_session) {
            let has_permission = permissions
                .get(&(user_id, project_id))
                .await
                .map(|v| permission_type <= v)
                .unwrap_or_default();
            if has_permission {
                log!(
                    "Permission already granted. for user_id: {}, project_id: {}",
                    user_id,
                    project_id
                );
                permission_allow()
            } else if let Some(permission) =
                request_project_permission(permissions, user_id, project_id).await?
            {
                if permission_type <= permission {
                    log!(
                        "Permission granted. for user_id: {}, project_id: {}",
                        user_id,
                        project_id
                    );
                    permission_allow()
                } else {
                    log!(
                        "Permission denied. for user_id: {}, project_id: {}",
                        user_id,
                        project_id
                    );
                    permission_denied("/user/projects")
                }
            } else {
                log!(
                    "Permission denied. for user_id: {}, project_id: {}",
                    user_id,
                    project_id
                );
                permission_denied("/user/projects")
            }
        } else {
            log!(
                "Permission denied. for anonymous: project_id: {}",
                project_id
            );
            permission_denied("/login")
        }
    }
    pub fn permission_allow() -> Result<PermissionResult, ServerFnError> {
        Ok(PermissionResult::Allow)
    }

    pub fn permission_denied(to_path: &str) -> Result<PermissionResult, ServerFnError> {
        Ok(PermissionResult::Redirect(to_path.to_string()))
    }
}
