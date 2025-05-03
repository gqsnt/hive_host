use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::LazyLock;
use std::thread::available_parallelism;
use tokio::net::TcpListener;
use tokio::runtime;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::server::server_children::run_unix_socket;
use common::Slug;
use hivehost_server_hosting::{accept_loop, cache_project_path, create_socket, HostingResult, CACHE, DB, TOKEN};
use hivehost_server_hosting::handler::{handle_command};

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
    let hosting_addr = dotenvy::var("HOSTING_ADDR")?;
    let server_hosting_socket_path = dotenvy::var("SERVER_HOSTING_SOCKET_PATH")?;
    let addr = SocketAddr::from_str(&hosting_addr)?;
    let socket = create_socket(addr)?;

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
    
    let _ = handle.spawn(async {
        let _ = run_unix_socket(server_hosting_socket_path, handle_command).await;
    }).await;

    let listener = TcpListener::from_std(socket.into())?;
    let addr = listener.local_addr()?;
    info!("Listening on: {}", addr);

    // spawn accept loop into a task so it is scheduled on the runtime with all the other tasks.
    let accept_loop = accept_loop(handle.clone(), listener);
    handle.spawn(accept_loop).await.unwrap()
}

