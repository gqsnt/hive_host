use common::Slug;
use futures::StreamExt;
use hivehost_server_hosting::handler::ServerToHostingServer;
use hivehost_server_hosting::{accept_hosting_loop, cache_project_path, create_socket, AppState, HostingResult, CACHE, DB, TOKEN};
use std::future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, LazyLock};
use std::thread::available_parallelism;
use secrecy::SecretString;
use tarpc::server;
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::hosting_command::tarpc::{ServerHosting, HOSTING_SOCKET_PATH};

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
        let unix_slug = project_slug.to_project_slug_str();
        cache_project_path(unix_slug).await;
    }
    drop(db);
    
    let _ = tokio::fs::remove_file(HOSTING_SOCKET_PATH).await;

    let server_auth = dotenvy::var("SERVER_AUTH").unwrap_or_else(|_| "hivehost".to_string());
    let connected = Arc::new(RwLock::new(false));
    let app_state = AppState {
        server_auth: Arc::new(SecretString::from(server_auth)),
        connected: connected.clone(),
    };
    
    

    
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 3002);
    let socket = create_socket(addr).expect("Failed to create socket");
    let listener = TcpListener::from_std(socket.into())?;
    let accept_hosting_loop = accept_hosting_loop(handle.clone(), listener);
    let mut listener =
        tarpc::serde_transport::unix::listen(HOSTING_SOCKET_PATH, Bincode::default)
            .await?;
    
    listener.config_mut().max_frame_length(10*10*1024);
    let (http_res, command_res) = tokio::join!(
        handle.spawn(accept_hosting_loop),
        handle.spawn(
            listener
                .filter_map(|r| future::ready(r.ok()))
                .map(server::BaseChannel::with_defaults)
                .map(move |channel| {
                    let server = ServerToHostingServer(app_state.clone());
                    channel
                        .execute(server.serve())
                        .for_each(|response| async move {
                            tokio::spawn(response);
                        })
                })
                .buffer_unordered(10)
                .for_each(|_| async {})
        )
    );
    if let Err(e) = http_res {
        error!("HTTP accept loop task failed: {:?}", e);
    }
    match command_res {
        Ok(_) => info!("Command listener finished gracefully."),
        Err(e) => error!("Command listener task failed: {:?}", e),
    }
    Ok(())
}
