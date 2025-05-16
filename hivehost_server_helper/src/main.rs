
use futures::StreamExt;
use hivehost_server_helper::command::ServerHelperServer;
use hivehost_server_helper::{ServerHelperResult, BTRFS_DEVICE};
use std::future;
use std::sync::LazyLock;
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use tarpc::{server};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::helper_command::tarpc::{ServerHelper, HELPER_SOCKET_PATH};

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

    info!("Server helper socket path: {}", HELPER_SOCKET_PATH);
    let mut listener =
        tarpc::serde_transport::unix::listen(HELPER_SOCKET_PATH, Bincode::default)
            .await?;
    listener.config_mut().max_frame_length(usize::MAX);
    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .map(|channel| {
            let server = ServerHelperServer;
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
