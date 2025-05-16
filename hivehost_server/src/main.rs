use axum::routing::post;
use axum::Router;
use hivehost_server::{connect_server_helper_client, connect_server_hosting_client, AppState, ServerResult, WebsiteToServerServer};
use moka::future::Cache;
use secrecy::SecretString;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use axum::extract::DefaultBodyLimit;
use futures::{future, StreamExt};
use tarpc::{server};
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use common::helper_command::tarpc::HELPER_SOCKET_PATH;
use common::hosting_command::tarpc::HOSTING_SOCKET_PATH;
use common::server_action::tarpc::WebsiteToServer;
use common::{SERVER_PORT, SERVER_TOKEN_PORT};
use common::tarpc_client::TarpcClient;
use hivehost_server::handle_token::{server_project_action_token};

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

    
    
    let server_helper_client = Arc::new(TarpcClient::new(HELPER_SOCKET_PATH.to_string(), None, connect_server_helper_client));
    let server_helper_client_to_connect = server_helper_client.clone();
    tokio::spawn(async move {
        if let Err(e) = server_helper_client_to_connect.connect().await {
            error!("Initial WebsiteServerClient connection failed: {:?}", e);
        }
    });
    
    
    let server_hosting_client = Arc::new(TarpcClient::new(HOSTING_SOCKET_PATH.to_string(), None, connect_server_hosting_client));
    let server_hosting_client_to_connect = server_hosting_client.clone();
    tokio::spawn(async move {
        if let Err(e) = server_hosting_client_to_connect.connect().await {
            error!("Initial WebsiteServerClient connection failed: {:?}", e);
        }
    });
    

    let app_state = AppState {
        project_token_action_cache: Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .build(),
        ),
        token_auth: token_action_auth,
        helper_client:server_helper_client,
        hosting_client:server_hosting_client,
        file_uploads: Arc::new( Cache::builder()
            .time_to_live(Duration::from_secs(3600))
            .build()),
        connected: Arc::new(tokio::sync::RwLock::new(false)),
    };

    
    let listener_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), SERVER_PORT);
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
    info!("Listener on {}", listener_addr);


    let token_app = Router::new()
        .route("/token/{token}", post(server_project_action_token))
        .layer(DefaultBodyLimit::max(65536000))
        .layer(CorsLayer::permissive())
        .with_state(app_state);


    let tcp_addr_token = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SERVER_TOKEN_PORT);
    let listener_token = tokio::net::TcpListener::bind(&tcp_addr_token).await?;
    info!("Token Listener on {}", tcp_addr_token);
    axum::serve(listener_token, token_app).await?;
    Ok(())
}
