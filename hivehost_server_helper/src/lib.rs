use secrecy::SecretString;
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;

pub mod command;

pub static BTRFS_DEVICE: LazyLock<String> =
    LazyLock::new(|| dotenvy::var("BTRFS_DEVICE").unwrap_or_else(|_| "/dev/sda".to_string()));

pub type ServerHelperResult<T> = Result<T, ServerHelperError>;

#[derive(Debug, thiserror::Error)]
pub enum ServerHelperError {
    #[error("IO Error {0}")]
    IoError(#[from] tokio::io::Error),
    #[error("Failed to execute command: {0}")]
    Other(String),
    #[error("Sanitize Error {0}")]
    SanitizeError(#[from] common::SanitizeError),
}

#[derive(Clone)]
pub struct AppState {
    pub server_auth: Arc<SecretString>,
    pub connected: Arc<RwLock<bool>>,
}
