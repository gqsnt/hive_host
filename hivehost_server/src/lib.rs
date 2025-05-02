pub mod helper_client;
pub mod project_action;
pub mod server_action;

use crate::helper_client::HelperClient;
use axum::extract::FromRef;
use axum::http::StatusCode;
use common::server_project_action::ServerProjectAction;
use common::{ProjectId, ProjectSlugStr, UserId};
use moka::future::Cache;
use secrecy::SecretString;
use std::path::StripPrefixError;
use std::sync::Arc;
use thiserror::Error;

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Tokio I/O error: {0}")]
    Io(#[from] tokio::io::Error),
    #[error("DotEnv error: {0}")]
    DotEnv(#[from] dotenvy::Error),
    #[error("AddrParse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("StripPrefix error: {0}")]
    StripPrefixError(#[from] StripPrefixError),
    #[error("Helper client error: {0}")]
    HelperClientError(#[from] helper_client::HelperClientError),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid path")]
    InvalidPath,
    #[error("Target not found")]
    TargetNotFound,
    #[error("Out of projects scope")]
    OutOfProjectsScope,
    #[error("Path is not a file")]
    PathIsNotFile,
    #[error("Path is not a directory")]
    PathIsNotDir,
    #[error("Path has no parent")]
    PathHasNoParent,
    #[error("Path is not a valid project path")]
    PathNotAValidProjectPath,
    #[error("Cant read file name {0}")]
    CantReadFileName(String),
}

impl From<ServerError> for (StatusCode, String) {
    fn from(value: ServerError) -> Self {
        let message = value.to_string();
        let status_code = match value {
            ServerError::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::error!("Server error: {}", message);
        (status_code, message)
    }
}

#[derive(Clone, Debug)]
pub struct ServerUserId(pub String);

#[derive(Clone, Debug)]
pub struct ServerProjectId(pub String);

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
    pub server_project_action_cache: Arc<Cache<String, (ProjectSlugStr, ServerProjectAction)>>,
    pub helper_client: HelperClient,
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
        return Err(ServerError::Unauthorized.into());
    }};
}
