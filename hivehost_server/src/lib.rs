pub mod handle_token;
pub mod project_action;
pub mod server_action;

use crate::project_action::handle_server_project_action;
use crate::server_action::handle_user_action;
use axum::extract::FromRef;
use axum::http::StatusCode;
use common::helper_command::tarpc::ServerHelperClient;
use common::hosting_command::tarpc::ServerHostingClient;
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::server_action::tarpc::WebsiteToServer;
use common::server_action::token_action::{TokenAction, TokenActionResponse};
use common::server_action::user_action::{ServerUserAction, ServerUserResponse};
use common::tarpc_client::{TarpcClient, TarpcClientError};
use common::{AuthResponse, AuthToken, ProjectId, ProjectSlugStr, SanitizeError, UserId, Validate};
use moka::future::Cache;
use secrecy::{ExposeSecret, SecretString};
use std::path::StripPrefixError;
use std::str::FromStr;
use std::sync::Arc;
use tarpc::context::Context;
use tarpc::tokio_serde::formats::Bincode;
use tarpc::{client, context};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

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
    #[error("Rpc error: {0}")]
    RpcError(#[from] client::RpcError),
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid path")]
    InvalidPath,
    #[error("Sanity check failed")]
    SanityCheckFailed,
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
    #[error("Invalid Message Length")]
    InvalidMessageLength,

    #[error("Tarpc Client Error {0}")]
    TarpcClientError(#[from] TarpcClientError),
    #[error("Sanitize Error {0}")]
    SanitizeError(#[from] SanitizeError),
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

pub type TarpcHelperClient = Arc<TarpcClient<ServerHelperClient>>;
pub type TarpcHostingClient = Arc<TarpcClient<ServerHostingClient>>;

pub type FileUploads = Arc<Cache<String, FileUpload>>;
pub type ProjectTokenActionCache = Arc<Cache<String, (ProjectSlugStr, TokenAction)>>;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub token_auth: SecretString,
    pub project_token_action_cache: ProjectTokenActionCache,
    pub helper_client: TarpcHelperClient,
    pub hosting_client: TarpcHostingClient,
    pub file_uploads: FileUploads,
    pub connected: Arc<RwLock<bool>>,
}

#[derive(Clone, Debug)]
pub struct FileUpload {
    pub file_name: String,
    pub file_path: String,
    pub project_slug: ProjectSlugStr,
}

#[derive(Clone)]
pub struct WebsiteToServerServer(pub AppState);
impl WebsiteToServer for WebsiteToServerServer {
    async fn token_action(
        self,
        _: Context,
        project_slug_str: ProjectSlugStr,
        action: TokenAction,
    ) -> TokenActionResponse {
        if !*self.0.connected.read().await {
            return TokenActionResponse::Error("Not connected".to_string());
        }
        if let Err(e) = project_slug_str.validate() {
            return TokenActionResponse::Error(format!("Invalid project slug: {e}"));
        };
        let token = Uuid::new_v4().to_string();
        info!(
            "Token action: {:?} for project: {:?}",
            action, project_slug_str
        );
        self.0
            .project_token_action_cache
            .insert(token.clone(), (project_slug_str.clone(), action))
            .await;
        TokenActionResponse::Ok(token)
    }

    async fn user_action(self, _: Context, action: ServerUserAction) -> ServerUserResponse {
        if !*self.0.connected.read().await {
            return ServerUserResponse::Error("Not connected".to_string());
        }
        if let Err(e) = action.validate() {
            return ServerUserResponse::Error(format!("Invalid action: {e}"));
        };
        handle_user_action(self.0.helper_client.clone(), action)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Error in user action: {}", e);
                ServerUserResponse::Error(e.to_string())
            })
    }

    async fn project_action(
        self,
        _: Context,
        project_slug: ProjectSlugStr,
        action: ProjectAction,
    ) -> ProjectResponse {
        if !*self.0.connected.read().await {
            return ProjectResponse::Error("Not connected".to_string());
        }
        if let Err(e) = action.validate() {
            return ProjectResponse::Error(format!("Invalid action: {e}"));
        };
        if let Err(e) = project_slug.validate() {
            return ProjectResponse::Error(format!("Invalid project slug: {e}"));
        };

        handle_server_project_action(
            self.0.hosting_client.clone(),
            self.0.helper_client.clone(),
            project_slug,
            action,
        )
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Error in project action: {}", e);
            ProjectResponse::Error(e.to_string())
        })
    }

    async fn auth(self, _: Context, token: AuthToken) -> AuthResponse {
        let mut connected = self.0.connected.write().await;
        if self.0.token_auth.expose_secret().eq(&token.0) {
            info!("Token auth success");
            *connected = true;
            AuthResponse::Ok
        } else {
            *connected = false;
            info!("Token auth failed");
            AuthResponse::Error
        }
    }
}

pub async fn connect_server_hosting_client(
    addr: String,
    token: String,
) -> Result<ServerHostingClient, TarpcClientError> {
    let mut transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default);
    transport.config_mut().max_frame_length(10 * 10 * 1024);
    let client = ServerHostingClient::new(client::Config::default(), transport.await?).spawn();
    match client
        .auth(context::current(), AuthToken::from_str(&token).unwrap())
        .await
    {
        Ok(AuthResponse::Ok) => Ok(client),
        _ => Err(TarpcClientError::ConnectionError("Auth failed".to_string())),
    }
}

pub async fn connect_server_helper_client(
    addr: String,
    token: String,
) -> Result<ServerHelperClient, TarpcClientError> {
    let mut transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default);
    transport.config_mut().max_frame_length(10 * 10 * 1024);
    let client = ServerHelperClient::new(client::Config::default(), transport.await?).spawn();
    match client
        .auth(context::current(), AuthToken::from_str(&token).unwrap())
        .await
    {
        Ok(AuthResponse::Ok) => Ok(client),
        _ => Err(TarpcClientError::ConnectionError("Auth failed".to_string())),
    }
}
