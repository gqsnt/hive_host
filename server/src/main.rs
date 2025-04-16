use axum::routing::post;
use axum::{Router};
use moka::future::Cache;
use secrecy::SecretString;
use server::project_action::{project_action_token, server_project_action};
use server::server_action::server_action;
use server::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenvy::dotenv().expect("Failed to load .env file");
    let token_action_auth =
        SecretString::from(dotenvy::var("TOKEN_AUTH").expect("TOKEN_AUTH must be set"));
    // build our application with a route
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
        .route("/token/{token}", post(project_action_token))
        .route("/server_action", post(server_action))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3002));
    tracing::debug!("listening on {addr}");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
