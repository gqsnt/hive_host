use axum::routing::post;
use axum::Router;
use hivehost_server::{AppState, ServerResult, WebsiteToServerServer};
use moka::future::Cache;
use secrecy::SecretString;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use futures::{future, StreamExt};
use tarpc::{client, server};
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use tower_http::cors::CorsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::server::tarpc_server_to_helper::ServerHelperClient;
use common::tarpc_hosting::ServerHostingClient;
use common::tarpc_website_to_server::WebsiteServer;
use hivehost_server::project_action::server_project_action_token;




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

    let server_addr_front = dotenvy::var("SERVER_ADDR_FRONT")?;

    let mut helper_transport = tarpc::serde_transport::unix::connect(server_helper_socket_path, Bincode::default);
    helper_transport.config_mut().max_frame_length(usize::MAX);
    let helper_client = ServerHelperClient::new(client::Config::default(), helper_transport.await?).spawn();
    
    let mut hosting_transport = tarpc::serde_transport::unix::connect(server_hosting_socket_path, Bincode::default);
    hosting_transport.config_mut().max_frame_length(usize::MAX);
    let hosting_client = ServerHostingClient::new(client::Config::default(), hosting_transport.await?).spawn();
    

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

    let listener_addr = dotenvy::var("SERVER_ADDR")?;
    
    let listener_addr = SocketAddr::from_str(&listener_addr)?;
    let mut website_server_listener = tarpc::serde_transport::tcp::listen(&listener_addr, Bincode::default).await?;
    website_server_listener.config_mut().max_frame_length(usize::MAX);

    let listener_state  = app_state.clone();
    tokio::spawn(async move {
        website_server_listener
            .filter_map(|r| future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .map(|channel| {
                let server = WebsiteToServerServer(listener_state.clone());
                channel
                    .execute(server.serve())
                    .for_each(|response| async move {
                        tokio::spawn(response);
                    })
            })
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;
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
