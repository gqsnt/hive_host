use crate::{AppResult};
use common::website_to_server::server_project_action::{ServerProjectAction, ServerProjectResponse};
use common::ProjectSlugStr;
use leptos::server;
use leptos::server_fn::codec::Bitcode;

pub fn token_url(server_url: &str, token: &str) -> String {
    format!("http://{server_url}/token/{token}")
}

#[server(input=Bitcode,output=Bitcode)]
pub async fn request_server_project_action_front(
    project_slug: ProjectSlugStr,
    action: ServerProjectAction,
    csrf: Option<String>,
) -> AppResult<ServerProjectResponse> {
    use crate::AppError;
    use common::website_to_server::{WebSiteToServerAction, WebSiteToServerResponse};
    use common::website_to_server::server_project_action::IsProjectServerAction;
    ssr::handle_project_permission_request(
        project_slug,
        action.permission(),
        action.require_csrf().then_some(csrf.unwrap_or_default()),
        |_, _, project_slug| async move {
            let mc = crate::ssr::multiplexer_client().unwrap();
            let response = mc.send(
                WebSiteToServerAction::from_server_project_action(project_slug.to_string(),action)
                ).await;

            match response {
                Ok(WebSiteToServerResponse::ServerProjectActionResponse(action_response)) => {
                    Ok(action_response)
                }
                _=> Err(AppError::Custom("server action failed".to_string())),
            }
        },
    )
    .await
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::security::ssr::AppAuthSession;
    use crate::security::utils::ssr::{get_auth_session_user_id, verify_easy_hash};
    use crate::ssr::{permissions, pool, Permissions};
    use crate::{AppError, AppResult};
    use common::website_to_server::permission::Permission;
    use common::{ProjectId, ProjectSlugStr, Slug, UserId};
    use leptos::logging::log;
    use sqlx::PgPool;
    use std::future::Future;
    use std::str::FromStr;

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct SqlPermission {
        pub(crate) permission: Permission,
    }

    pub async fn request_project_permission(
        permissions: Permissions,
        user_id: UserId,
        project_id: ProjectId,
    ) -> AppResult<Option<Permission>> {
        let pool = pool()?;
        let permission = sqlx::query_as!(
    SqlPermission,
    r#"SELECT permission as "permission: Permission" FROM permissions WHERE user_id = $1 AND project_id = $2"#,
    user_id,
    project_id
)
            .fetch_optional(&pool)
            .await
            .map_err(|_| AppError::UnauthorizedProjectAccess)?;
        if let Some(permission) = permission {
            let permission = permission.permission;
            permissions.insert((user_id, project_id), permission).await;
            Ok(Some(permission))
        } else {
            Ok(None)
        }
    }

    pub async fn handle_project_permission_request<F, Fut, T>(
        project_slug_str: ProjectSlugStr,
        required_permission: Permission,
        csrf: Option<String>,
        handler: F, // The closure containing specific logic
    ) -> AppResult<T>
    where
        F: FnOnce(AppAuthSession, PgPool, Slug) -> Fut,
        Fut: Future<Output = AppResult<T>>,
    {
        let auth = crate::ssr::auth(false)?;
        let server_vars = crate::ssr::server_vars()?;
        if let Some(csrf) = csrf {
            verify_easy_hash(
                auth.session.get_session_id().to_string(),
                server_vars.csrf_server.to_secret(),
                csrf,
            )?;
        }
        let project_id = Slug::from_str(project_slug_str.as_str())?.id;
        ensure_permission(&auth, project_id, required_permission).await?;
        let pool = pool()?;
        let project = sqlx::query!(r#"SELECT id, name FROM projects WHERE id = $1"#, project_id)
            .fetch_one(&pool)
            .await
            .map_err(|_| AppError::UnauthorizedProjectAccess)?;

        let full_project_slug = Slug::new(project.id, project.name);

        handler(auth, pool, full_project_slug).await
    }

    pub async fn ensure_permission(
        auth_session: &AppAuthSession,
        project_id: ProjectId,
        permission_type: Permission,
    ) -> AppResult<()> {
        let permissions = permissions()?;
        if let Some(user_id) = get_auth_session_user_id(auth_session) {
            let has_cached_permission = permissions
                .get(&(user_id, project_id))
                .await
                .map(|v| v.has_permission(&permission_type))
                .unwrap_or_default();
            if has_cached_permission {
                Ok(())
            } else if let Some(db_permission) =
                request_project_permission(permissions, user_id, project_id).await?
            {
                if db_permission.has_permission(&permission_type) {
                    log!(
                        "Permission granted. for user_id: {}, project_id: {}",
                        user_id,
                        project_id
                    );
                    Ok(())
                } else {
                    log!(
                        "Permission denied. for user_id: {}, project_id: {}",
                        user_id,
                        project_id
                    );
                    Err(AppError::UnauthorizedProjectAccess)
                }
            } else {
                log!(
                    "Permission denied. for user_id: {}, project_id: {}",
                    user_id,
                    project_id
                );
                Err(AppError::UnauthorizedProjectAccess)
            }
        } else {
            log!(
                "Permission denied. for anonymous: project_id: {}",
                project_id
            );
            Err(AppError::UnauthorizedAuthAccess)
        }
    }
}
