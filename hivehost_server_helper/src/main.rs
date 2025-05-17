use common::helper_command::tarpc::{HELPER_SOCKET_PATH, ServerHelper};
use futures::StreamExt;
use hivehost_server_helper::command::ServerHelperServer;
use hivehost_server_helper::{AppState, BTRFS_DEVICE, ServerHelperResult};
use secrecy::SecretString;
use std::future;
use std::sync::{Arc, LazyLock};
use tarpc::server;
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> ServerHelperResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(true))
        .init();

    LazyLock::force(&BTRFS_DEVICE);
    let _ = tokio::fs::remove_file(HELPER_SOCKET_PATH).await;
    let server_auth = dotenvy::var("SERVER_AUTH").unwrap_or_else(|_| "hivehost".to_string());
    let connected = Arc::new(RwLock::new(false));
    let app_state = AppState {
        server_auth: Arc::new(SecretString::from(server_auth)),
        connected: connected.clone(),
    };

    info!("Server helper socket path: {}", HELPER_SOCKET_PATH);
    let mut listener =
        tarpc::serde_transport::unix::listen(HELPER_SOCKET_PATH, Bincode::default).await?;
    listener.config_mut().max_frame_length(10 * 10 * 1024);
    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .map(|channel| {
            let server = ServerHelperServer(app_state.clone());
            channel
                .execute(server.serve())
                .for_each(|response| async move {
                    tokio::spawn(response);
                })
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;
    Ok(())
}
