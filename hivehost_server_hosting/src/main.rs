use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::LazyLock;
use std::thread::available_parallelism;
use async_trait::async_trait;
use tokio::net::TcpListener;
use tokio::runtime;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::multiplex_listener::{run_server_unix, RequestHandler};
use common::multiplex_protocol::{GenericRequest, GenericResponse};
use common::server::server_to_hosting::{ServerToHostingAction, ServerToHostingResponse};
use common::Slug;
use hivehost_server_hosting::{accept_hosting_loop, cache_project_path, create_socket, HostingResult, CACHE, DB, TOKEN};
use hivehost_server_hosting::handler::{handle_command};


#[derive(Clone)]
struct HostingRequestHandler;

#[derive(Default)]
struct HostingConnectionState;

#[async_trait]
impl RequestHandler<ServerToHostingAction, ServerToHostingResponse> for HostingRequestHandler {
    type ConnectionState = HostingConnectionState;

    async fn handle_request(
        &self,
        request: GenericRequest<ServerToHostingAction>,
        _conn_state: &mut Self::ConnectionState,
    ) -> GenericResponse<ServerToHostingResponse> {
        // Directly call the existing command handler logic
        handle_command(request).await
    }
}

pub fn main() -> HostingResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    LazyLock::force(&CACHE);
    LazyLock::force(&TOKEN);
    LazyLock::force(&DB);
    let cpus = available_parallelism()?.get();
    let runtime = runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(cpus)
        .build()?;
    runtime.block_on(serve(runtime.handle()))
}

async fn serve(handle: &runtime::Handle) -> HostingResult<()> {
    let db = DB.get().await.expect("DB must exist");
    let query =
        "SELECT id,name, active_snapshot_id FROM projects where active_snapshot_id is not null";
    let statement = db.prepare_cached(query).await?;
    let row = db.query(&statement, &[]).await?;
    info!("Found {} projects", row.len());
    for row in row {
        let name = row.get::<_, String>("name");
        let id = row.get::<_, i64>("id");
        let project_slug = Slug::new(id, name);
        let unix_slug = project_slug.to_string();
        cache_project_path(unix_slug).await;
    }
    drop(db);

    let server_hosting_socket_path = dotenvy::var("SERVER_HOSTING_SOCKET_PATH")?;
    let _ = tokio::fs::remove_file(server_hosting_socket_path.clone()).await;
    
    let hosting_addr = dotenvy::var("HOSTING_ADDR")?;
    let addr = SocketAddr::from_str(&hosting_addr)?;
    let socket = create_socket(addr).expect("Failed to create socket");
    let listener = TcpListener::from_std(socket.into())?;
    let accept_hosting_loop = accept_hosting_loop(handle.clone(), listener);
    let accept_command_loop = run_server_unix(server_hosting_socket_path, HostingRequestHandler);
    let (http_res, command_res) = tokio::join!(
        handle.spawn(accept_hosting_loop),
        handle.spawn(accept_command_loop)
    );
    if let Err(e) = http_res {
        error!("HTTP accept loop task failed: {:?}", e);
    }
    match command_res {
        Ok(Ok(())) => info!("Command listener finished gracefully."),
        Ok(Err(e)) => error!("Command listener failed: {:?}", e),
        Err(e) => error!("Command listener task failed: {:?}", e),
    }
    Ok(())
    
}

