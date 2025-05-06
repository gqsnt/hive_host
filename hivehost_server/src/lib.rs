pub mod project_action;
pub mod server_action;


use crate::project_action::handle_server_project_action;
use crate::server_action::handle_server_action;
use axum::extract::FromRef;
use axum::http::StatusCode;
use common::hosting::{HostingAction, HostingResponse};
use common::website_to_server::server_action::{ServerAction, ServerActionResponse};
use common::website_to_server::server_project_action::{
    ServerProjectAction, ServerProjectResponse,
};
use common::{ProjectId, ProjectSlugStr, UserId};
use moka::future::Cache;
use secrecy::SecretString;
use std::path::StripPrefixError;
use std::sync::Arc;
use tarpc::{client};
use tarpc::context::Context;
use tarpc::tokio_serde::formats::Bincode;
use thiserror::Error;
use common::server::tarpc_server_to_helper::ServerHelperClient;
use common::tarpc_client::{TarpcClient, TarpcClientError};
use common::tarpc_hosting::ServerHostingClient;
use common::tarpc_website_to_server::{WebsiteServer};

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
    RpcError(#[from] tarpc::client::RpcError),
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
    #[error("Invalid Message Length")]
    InvalidMessageLength,
    
    #[error("Tarpc Client Error {0}")]
    TarpcClientError(#[from] common::tarpc_client::TarpcClientError),
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

#[derive(Clone, FromRef)]
pub struct AppState {
    pub token_auth: SecretString,
    pub server_project_action_cache: Arc<Cache<String, (ProjectSlugStr, ServerProjectAction)>>,
    pub helper_client: TarpcHelperClient,
    pub hosting_client: TarpcHostingClient,
}

#[derive(Clone)]
pub struct WebsiteToServerServer(pub AppState);
impl WebsiteServer for WebsiteToServerServer {
    async fn server_action(self, _: Context, action: ServerAction) -> ServerActionResponse {
        handle_server_action(self.0, action)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Error handling server action: {:?}", e);
                ServerActionResponse::Error(e.to_string())
            })
    }

    async fn server_project_action(
        self,
        _: Context,
        project_slug: String,
        action: ServerProjectAction,
    ) -> ServerProjectResponse {
        handle_server_project_action(self.0, project_slug, action)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Error handling server project action: {:?}", e);
                ServerProjectResponse::Error(e.to_string())
            })
    }

    async fn hosting_action(
        self,
        _: Context,
        project_slug: String,
        action: HostingAction,
    ) -> HostingResponse {
        tracing::info!("Hosting action: {:?}", action);
        self.0.hosting_client.execute(|c, cx| async move {
            c.execute(cx,project_slug, action).await
        }).await
            .unwrap_or_else(|e| {
                tracing::error!("Error handling hosting action: {:?}", e);
                HostingResponse::Error(e.to_string())
            })
    }
}


pub async fn connect_server_hosting_client(addr: String) -> Result<ServerHostingClient, TarpcClientError> {
    let mut transport = tarpc::serde_transport::unix::connect(addr, Bincode::default);
    transport
        .config_mut()
        .max_frame_length(usize::MAX);
    Ok(ServerHostingClient::new(client::Config::default(), transport.await?).spawn())

}

pub async fn connect_server_helper_client(addr: String) -> Result<ServerHelperClient, TarpcClientError> {
    let mut transport = tarpc::serde_transport::unix::connect(addr, Bincode::default);
    transport
        .config_mut()
        .max_frame_length(usize::MAX);
    Ok(ServerHelperClient::new(client::Config::default(), transport.await?).spawn())

}
