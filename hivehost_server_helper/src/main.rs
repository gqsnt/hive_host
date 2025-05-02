use std::sync::LazyLock;
use tracing::{error, info};
use hivehost_server_helper::{handler, ServerHelperError, ServerHelperResult, BTRFS_DEVICE};
use listenfd::ListenFd;
use tokio::net::UnixListener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> ServerHelperResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_env("/home/canarit/projects/hive_host/.env")
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(true))
        .init();
    dotenvy::from_path("/home/canarit/projects/hive_host/.env").expect("Failed to load .env file");
    LazyLock::force(&BTRFS_DEVICE);
    let mut listen_fds = ListenFd::from_env();
    info!("HiveHost Helper Service starting...");
    let unix_listener_std = match listen_fds.take_unix_listener(0)? { // Use listen_fds result directly
        Some(listener) => {
            listener // This is a std::os::unix::net::UnixListener
        },
        None => {
            error!("No listener socket received from systemd. Ensure service is run via hivehost_server_helper.socket.");
            return Err(ServerHelperError::Other("No listener socket received from systemd".to_string()));
        }
    };
    unix_listener_std.set_nonblocking(true)?;
    let listener = UnixListener::from_std(unix_listener_std)?;
    info!("Listening for connections on systemd socket...");
    loop{
        match listener.accept().await{
            Ok((stream, _)) => {
                info!("Accepted connection from {:?}", stream.peer_addr());
                tokio::spawn(handler::handle_connection(stream));
                
            },
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        };
    }
}