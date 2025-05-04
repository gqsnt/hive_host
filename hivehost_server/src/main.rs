use axum::routing::post;
use axum::Router;
use hivehost_server::{AppState, MultiplexServerHelperClient, MultiplexServerHostingClient, ServerResult};
use moka::future::Cache;
use secrecy::SecretString;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::multiplex_listener::{run_server_tcp};
use hivehost_server::project_action::server_project_action_token;
use hivehost_server::request_handler::{ServerRequestHandler};

#[tokio::main]
async fn main() -> ServerResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let token_action_auth = SecretString::from(dotenvy::var("TOKEN_AUTH")?);
    let server_helper_socket_path = dotenvy::var("SERVER_HELPER_SOCKET_PATH")?;
    let server_hosting_socket_path = dotenvy::var("SERVER_HOSTING_SOCKET_PATH")?;
    // build our application with a route
    let server_addr = dotenvy::var("SERVER_ADDR")?;
    let server_addr_front = dotenvy::var("SERVER_ADDR_FRONT")?;
    
    
    let helper_client = MultiplexServerHelperClient::new(
        server_helper_socket_path,
        None,
        5
    )?;
    let hosting_client = MultiplexServerHostingClient::new(
        server_hosting_socket_path,
        None,
        5
    )?;

    let app_state = AppState {
        server_project_action_cache: Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .build(),
        ),
        token_auth: token_action_auth,
        helper_client,
        hosting_client,
    };

    let listener_addr = server_addr; // Or a different address/port if needed
    let listener_state = app_state.clone();
    tokio::spawn(async move {
        let handler = ServerRequestHandler { state: listener_state }; // Create handler
        if let Err(e) = run_server_tcp(listener_addr, handler).await { // Use run_server
            eprintln!("Multiplex Listener failed: {e}");
        }
    });
    
    let app = Router::new()
        .route("/token/{token}", post(server_project_action_token))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let tcp_addr_front = SocketAddr::from_str(&server_addr_front)?;
    let listener = tokio::net::TcpListener::bind(tcp_addr_front).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
