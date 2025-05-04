use hivehost_server_helper::{ServerHelperResult, BTRFS_DEVICE};
use std::sync::LazyLock;
use async_trait::async_trait;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::multiplex_listener::{run_server_unix, RequestHandler};
use common::multiplex_protocol::{GenericRequest, GenericResponse};
use common::server::server_to_helper::{ServerToHelperAction, ServerToHelperResponse};
use hivehost_server_helper::command::handle_command;

#[derive(Clone)]
struct HelperRequestHandler;

#[derive(Default)] // Simple state, maybe just unit ()
struct HelperConnectionState;

#[async_trait]
impl RequestHandler<ServerToHelperAction, ServerToHelperResponse> for HelperRequestHandler {
    type ConnectionState = HelperConnectionState; // Use the simple state

    async fn handle_request(
        &self,
        request: GenericRequest<ServerToHelperAction>,
        _conn_state: &mut Self::ConnectionState, // State not used here
    ) -> GenericResponse<ServerToHelperResponse> {
        // Directly call the existing command handler logic
        handle_command(request).await
    }
}

#[tokio::main]
async fn main() -> ServerHelperResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_env("/home/canarit/projects/hive_host/.env")
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(true))
        .init();
    
    LazyLock::force(&BTRFS_DEVICE);

    let server_helper_socket_path = dotenvy::var("SERVER_HELPER_SOCKET_PATH").expect("HELPER_ADDR not set");
    info!("Server helper socket path: {}", server_helper_socket_path);
    let _ = tokio::fs::remove_file(server_helper_socket_path.clone()).await;
    run_server_unix(server_helper_socket_path, HelperRequestHandler).await?;
    Ok(())
}
