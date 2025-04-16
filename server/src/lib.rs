pub mod cmd;
pub mod project_action;
pub mod server_action;

use axum::extract::FromRef;
use common::server_project_action::ServerProjectAction;
use common::{ProjectId, ProjectUnixSlugStr, UserId};
use moka::future::Cache;
use secrecy::SecretString;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServerUserId(pub String);

#[derive(Clone, Debug)]
pub struct ServerProjectId (pub String);


impl From<UserId> for ServerUserId {
    fn from(user_id: UserId) -> Self {
        ServerUserId(user_id.to_string().replace("-", ""))
    }
}

impl From<ProjectId> for ServerProjectId {
    fn from(project_id: ProjectId) -> Self {
        ServerProjectId(project_id.to_string().replace("-", ""))
    }
}



#[derive(Clone, FromRef)]
pub struct AppState {
    pub token_auth: SecretString,
    pub server_project_action_cache: Arc<Cache<String, (ProjectUnixSlugStr, ServerProjectAction)>>,
}

#[macro_export]
macro_rules! ensure_authorization {
    ($headers:expr, $state:expr, $success:block) => {{
        if let Some(auth) = $headers.get(axum::http::header::AUTHORIZATION) {
            if let Ok(auth_str) = auth.to_str() {
                if auth_str
                    .trim_start_matches("Bearer ")
                    .eq($state.token_auth.expose_secret())
                {
                    return $success;
                }
            }
        }
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }};
}
