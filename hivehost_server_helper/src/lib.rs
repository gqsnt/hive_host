use std::sync::LazyLock;

pub mod command;

pub static BTRFS_DEVICE: LazyLock<String> =
    LazyLock::new(|| dotenvy::var("BTRFS_DEVICE").unwrap_or_else(|_| "/dev/sda".to_string()));

pub type ServerHelperResult<T> = Result<T, ServerHelperError>;

#[derive(Debug, thiserror::Error)]
pub enum ServerHelperError {
    #[error("Unix Stream Error {0}")]
    UnixStreamError(#[from] common::multiplex_listener::MultiplexListenerError),
    #[error("IO Error {0}")]
    IoError(#[from] tokio::io::Error),
    #[error("Failed to execute command: {0}")]
    Other(String),
}
