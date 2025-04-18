use leptos::prelude::ServerFnError;
use leptos::server;
use std::str::FromStr;

use common::permission::Permission;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use common::{ProjectId, ProjectSlug, ProjectSlugStr, UserId};

pub fn token_url(token: String) -> String {
    format!("http://127.0.1:3002/token/{}", token)
}

#[server]
pub async fn request_server_project_action(
    project_slug: ProjectSlugStr,
    action: ServerProjectAction,
) -> Result<ServerProjectActionResponse, ServerFnError> {
    use common::server_project_action::{IsProjectServerAction, ServerProjectActionRequest};
    use secrecy::ExposeSecret;

    let project_slug = ProjectSlug::from_str(project_slug.as_str())
        .map_err(|_| {
            leptos_axum::redirect("/user/projects");
            ServerFnError::new("Invalid project slug")
        })?;
    

    let auth = crate::ssr::auth(false)?;
    ssr::ensure_permission(&auth, project_slug.id, action.permission()).await?;
    let server_vars = crate::ssr::server_vars()?;
    let client = reqwest::Client::new();
    let with_token = action.with_token();
    let mut req = ServerProjectActionRequest {
        token: None,
        action,
        project_slug,
    };
    if with_token {
        let token = sqlx::types::Uuid::new_v4().to_string();
        req.token = Some(token.clone());
        let _ = client
            .post("http://127.0.0.1:3002/server_project_action")
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?;
        Ok(ServerProjectActionResponse::Token(token))
    } else {
        Ok(client
            .post("http://127.0.0.1:3002/server_project_action")
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .json::<ServerProjectActionResponse>()
            .await?)
    }
}

pub enum PermissionResult {
    Allow,
    Redirect(String),
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::security::permission::PermissionResult;
    use crate::security::ssr::AppAuthSession;
    use crate::security::utils::ssr::get_auth_session_user_id;
    use crate::ssr::{permissions, pool, Permissions};
    use common::permission::Permission;
    use common::{ProjectId, UserId};
    use leptos::logging::log;
    use leptos::prelude::ServerFnError;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, sqlx::Type)]
    #[sqlx(type_name = "permission_type", rename_all = "lowercase")]
    pub enum SqlPermissionType {
        Read = 0,
        Write,
        Owner,
    }

    impl From<SqlPermissionType> for Permission {
        fn from(value: SqlPermissionType) -> Self {
            match value {
                SqlPermissionType::Read => Permission::Read,
                SqlPermissionType::Write => Permission::Write,
                SqlPermissionType::Owner => Permission::Owner,
            }
        }
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct SqlPermission {
        pub(crate) permission: SqlPermissionType,
    }

    pub async fn request_project_permission(
        permissions: Permissions,
        user_id: UserId,
        project_id: ProjectId,
    ) -> Result<Option<Permission>, ServerFnError> {
        let pool = pool()?;
        let permission = sqlx::query_as!(
    SqlPermission,
    r#"SELECT permission as "permission: SqlPermissionType" FROM permissions WHERE user_id = $1 AND project_id = $2"#,
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
