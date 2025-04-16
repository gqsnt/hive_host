use std::net::SocketAddr;
use memory_serve::{load_assets, CacheControl, MemoryServe};
use secrecy::SecretString;
use tower_http::compression::{CompressionLayer, Predicate};
use tower_http::compression::predicate::{NotForContentType, SizeAbove};
use tower_http::CompressionLevel;
use common::UserId;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use website::app::*;
    use website::auth::User;
    use website::ssr::{leptos_routes_handler, server_fn_handler, AppState};
    use axum::{routing::get, Router};
    use axum_session_auth::{AuthConfig, AuthSessionLayer};
    use axum_session_sqlx::SessionPgPool;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::PgPool;
    use std::sync::Arc;
    use website::rate_limiter::ssr::{rate_limit_middleware, RateLimiter};
    use website::ssr::ServerVars;
    use website::tasks::refresh_server_csrf::RefreshServerCsrf;
    use website::tasks::ssr::TaskDirector;
    use axum_session::{SessionConfig, SessionLayer, SessionStore};
    use moka::future::Cache;
    use std::time::Duration;

    dotenvy::dotenv().ok();

    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let token_action_auth = SecretString::from(
        dotenvy::var("TOKEN_AUTH").expect("TOKEN_AUTH must be set")
    );
    let pool = PgPool::connect(database_url.as_str())
        .await
        .expect("Could not connect to database");
    sqlx::migrate!().run(&pool).await.expect("Migration failed");
    let session_config = SessionConfig::default().with_table_name("sessions");
    let session_store =
        SessionStore::<SessionPgPool>::new(Some(pool.clone().into()), session_config)
            .await
            .unwrap();

    let auth_config = AuthConfig::<UserId>::default();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);
    let server_vars = ServerVars::new(token_action_auth);
    let csrf_server = server_vars.csrf_server.clone();

    let rate_limiter = Arc::new(RateLimiter::default());

    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        pool: pool.clone(),
        routes: routes.clone(),
        rate_limiter: rate_limiter.clone(),
        server_vars,
        permissions: Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(900))
                .build(),
        ), // cache for 15 minutes
    };

    let mut task_director = TaskDirector::default();
    task_director.add_task(RefreshServerCsrf::new(csrf_server, 0, false));
    tokio::spawn(async move {
        task_director.run().await;
    });

    let app = Router::new()
        // .nest(
        //     "/assets",
        //     MemoryServe::new(load_assets!("./target/site/assets"))
        //         .enable_brotli(!cfg!(debug_assertions))
        //         .cache_control(CacheControl::Custom("public, max-age=31536000"))
        //         .into_router::<AppState>()
        // )
        // .nest(
        //     "/pkg",
        //     MemoryServe::new(load_assets!("./target/site/pkg"))
        //         .enable_brotli(!cfg!(debug_assertions))
        //         .cache_control(CacheControl::Custom("public, max-age=31536000"))
        //         .into_router::<AppState>()
        // )
        .route(
            "/api/{*wildcard}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(
            CompressionLayer::new()
                .br(true)
                .zstd(true)
                .quality(CompressionLevel::Default)
                .compress_when(
                    SizeAbove::new(256)
                        .and(NotForContentType::GRPC)
                        .and(NotForContentType::IMAGES)
                        .and(NotForContentType::SSE)
                        .and(NotForContentType::const_new("text/javascript"))
                        .and(NotForContentType::const_new("application/wasm"))
                        .and(NotForContentType::const_new("text/css")),
                ),
        )
        .layer(
            AuthSessionLayer::<User, UserId, SessionPgPool, PgPool>::new(Some(pool.clone()))
                .with_config(auth_config),
        )
        .layer(SessionLayer::new(session_store))
        .route_layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ))
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
