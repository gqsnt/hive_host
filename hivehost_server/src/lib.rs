pub mod project_action;
pub mod server_action;
pub mod handle_token;

use crate::project_action::handle_server_project_action;
use crate::server_action::handle_user_action;
use axum::extract::FromRef;
use axum::http::StatusCode;
use common::helper_command::tarpc::ServerHelperClient;
use common::hosting_command::tarpc::ServerHostingClient;
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::server_action::tarpc::WebsiteToServer;
use common::server_action::user_action::{ServerUserAction, ServerUserResponse};
use common::tarpc_client::{TarpcClient, TarpcClientError};
use common::{ProjectId, ProjectSlugStr, UserId};
use moka::future::Cache;
use secrecy::SecretString;
use std::path::StripPrefixError;
use std::sync::Arc;
use async_broadcast::{Receiver, Sender};
use dashmap::DashMap;
use futures::Stream;
use tarpc::context::Context;
use tarpc::tokio_serde::formats::Bincode;
use tarpc::{client};
use thiserror::Error;
use tracing::info;
use common::server_action::token_action::{TokenAction, TokenActionResponse};
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

pub type FileUploads = Arc<Cache<String, FileUpload>>;
pub type ProjectTokenActionCache = Arc<Cache<String, (ProjectSlugStr, TokenAction)>>;


#[derive(Clone, FromRef)]
pub struct AppState {
    pub token_auth: SecretString,
    pub project_token_action_cache: ProjectTokenActionCache,
    pub helper_client: TarpcHelperClient,
    pub hosting_client: TarpcHostingClient,
    pub file_uploads: FileUploads,
}


#[derive(Clone, Debug)]
pub struct FileUpload{
    pub file_name: String,
    pub file_path: String,
    pub project_slug:ProjectSlugStr,
    pub total: usize,
    pub tx:Sender<usize>,
    pub rx:Receiver<usize>,
}

pub async fn get_file_upload(file_uploads:FileUploads, token:String, info:Option<(ProjectSlugStr,String ,String)>) -> impl Stream<Item=usize>{
    let entry = file_uploads.entry(token.clone()).or_insert_with(async {
        let (tx, rx) = async_broadcast::broadcast(128);
        let (project_slug, file_name, file_path) = info.unwrap_or_default();
        FileUpload {
            file_name,
            file_path,
            project_slug,
            total: 0,
            tx,
            rx,
        }
    }).await;
    entry.value().rx.clone()
}


#[derive(Clone)]
pub struct WebsiteToServerServer(pub AppState);
impl WebsiteToServer for WebsiteToServerServer {
    async fn token_action(self, _: Context, project_slug_str: ProjectSlugStr, action: TokenAction) -> TokenActionResponse {
        let token = Uuid::new_v4().to_string();
        info!("Token action: {:?} for project: {}", action, project_slug_str);
        self.0.project_token_action_cache.insert(
            token.clone(),
            (project_slug_str.clone(), action)
        ).await;
        token
    }

    async fn user_action(self, _: Context, action: ServerUserAction) -> ServerUserResponse {
        handle_user_action(
            self.0.helper_client.clone(),
            action).await
            .unwrap_or_else(|e| {
                tracing::error!("Error in user action: {}", e);
                ServerUserResponse::Error(e.to_string())
            })
    }

    async fn project_action(
        self,
        _: Context,
        project_slug: String,
        action: ProjectAction,
    ) -> ProjectResponse {
        handle_server_project_action(
            self.0.hosting_client.clone(),
            self.0.helper_client.clone(),  
            project_slug,
            action).await
            .unwrap_or_else(|e| {
                tracing::error!("Error in project action: {}", e);
                ProjectResponse::Error(e.to_string())
            })
        
    }
}

pub async fn connect_server_hosting_client(
    addr: String,
) -> Result<ServerHostingClient, TarpcClientError> {
    let mut transport = tarpc::serde_transport::unix::connect(addr, Bincode::default);
    transport.config_mut().max_frame_length(usize::MAX);
    Ok(ServerHostingClient::new(client::Config::default(), transport.await?).spawn())
}

pub async fn connect_server_helper_client(
    addr: String,
) -> Result<ServerHelperClient, TarpcClientError> {
    let mut transport = tarpc::serde_transport::unix::connect(addr, Bincode::default);
    transport.config_mut().max_frame_length(usize::MAX);
    Ok(ServerHelperClient::new(client::Config::default(), transport.await?).spawn())
}


