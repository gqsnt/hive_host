pub mod command;
pub mod handler;


pub type ServerHelperResult<T> = Result<T, ServerHelperError>;
pub const USER_GROUP: &str = "sftp_users";

#[derive(Debug, thiserror::Error)]
pub enum ServerHelperError {
    #[error("Failed to execute command: {0}")]
    Io(#[from] tokio::io::Error),
    #[error("Failed to execute command: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Failed to execute command: {0}")]
    Other(String),
}

