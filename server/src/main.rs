use axum::routing::post;
use axum::Router;
use moka::future::Cache;
use secrecy::SecretString;
use server::project_action::{server_project_action, server_project_action_token};
use server::server_action::server_action;
use server::{AppState, ServerResult};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> ServerResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenvy::dotenv().expect("Failed to load .env file");
    let token_action_auth = SecretString::from(dotenvy::var("TOKEN_AUTH")?);
    // build our application with a route
    let server_addr = dotenvy::var("SERVER_ADDR")?;
    let addr = SocketAddr::from_str(&server_addr)?;

    let app_state = AppState {
        server_project_action_cache: Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .build(),
        ),
        token_auth: token_action_auth,
    };
    let app = Router::<AppState>::new()
        .route("/server_project_action", post(server_project_action))
        .route("/token/{token}", post(server_project_action_token))
        .route("/server_action", post(server_action))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    tracing::debug!("listening on {addr}");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
