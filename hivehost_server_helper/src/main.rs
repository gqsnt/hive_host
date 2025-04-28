use tracing::{error, info, Level};
use tracing_subscriber;
use hivehost_server_helper::{handler, ServerHelperError, ServerHelperResult};
use dotenvy;
use listenfd::ListenFd;
use tokio::net::UnixListener;

#[tokio::main]
async fn main() -> ServerHelperResult<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO) // Adjust log level as needed
        .with_target(true)
        .init();
    dotenvy::from_path("/home/canarit/projects/hive_host/.env").expect("Failed to load .env file");
    let mut listen_fds = ListenFd::from_env();
    let server_helper_socket_path = dotenvy::var("SERVER_HELPER_SOCKET_PATH").expect("SERVER_HELPER_SOCKET_PATH not found");
    info!("HiveHost Helper Service starting...");
    let unix_listener_std = match listen_fds.take_unix_listener(0)? { // Use listen_fds result directly
        Some(listener) => {
            info!("Successfully obtained listener socket from systemd.");
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