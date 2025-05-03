use hivehost_server_helper::{ServerHelperResult, BTRFS_DEVICE};
use std::sync::LazyLock;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::server::server_children::run_unix_socket;
use hivehost_server_helper::command::handle_command;

#[tokio::main]
async fn main() -> ServerHelperResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_env("/home/canarit/projects/hive_host/.env")
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(true))
        .init();
    let server_helper_socket_path = dotenvy::var("SERVER_HELPER_SOCKET_PATH").expect("HELPER_ADDR not set");
    LazyLock::force(&BTRFS_DEVICE);
    run_unix_socket(server_helper_socket_path, handle_command).await?;
    Ok(())
}
