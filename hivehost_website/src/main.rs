


#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> hivehost_website::AppResult<()> {
    use leptos::logging::error;
    use common::tarpc_client::TarpcClient;
    use hivehost_website::ssr::{connect_website_client};
    use hivehost_website::github::ssr::github_post_install_callback;
    use axum::{routing::get, Router};
    use axum_session::{SessionConfig, SessionLayer, SessionStore};
    use axum_session_auth::{AuthConfig, AuthSessionLayer};
    use axum_session_sqlx::SessionPgPool;
    use common::UserId;
    use hivehost_website::app::*;
    use hivehost_website::models::User;
    use hivehost_website::rate_limiter::ssr::RateLimiter;
    use hivehost_website::ssr::ServerVars;
    use hivehost_website::ssr::{leptos_routes_handler, server_fn_handler, AppState};
    use hivehost_website::tasks::refresh_server_csrf::RefreshServerCsrf;
    use hivehost_website::tasks::ssr::TaskDirector;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use moka::future::Cache;
    use sqlx::PgPool;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;
    use hivehost_website::github::ssr::github_webhook;
    use std::net::{IpAddr, Ipv4Addr};
    use axum::routing::post;
    use dashmap::DashMap;
    use common::server_action::tarpc::WebsiteToServerClient;
    use common::SERVER_PORT;
    
    
    dotenvy::dotenv().ok();

    let database_url = dotenvy::var("DATABASE_URL")?;
    let pool = PgPool::connect(database_url.as_str())
        .await
        .expect("Could not connect to database");
    sqlx::migrate!().run(&pool).await?;
    let session_config = SessionConfig::default().with_table_name("sessions");
    let session_store =
        SessionStore::<SessionPgPool>::new(Some(pool.clone().into()), session_config).await?;

    let auth_config = AuthConfig::<UserId>::default().with_anonymous_user_id(Some(-1));

    let mut conf = get_configuration(None).unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 3000);
    conf.leptos_options.site_addr = addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let github_client_id= dotenvy::var("GITHUB_CLIENT_ID")?;
    let routes = generate_route_list(App);
    let server_vars = ServerVars::new(github_client_id);
    let csrf_server = server_vars.csrf_server.clone();

    let rate_limiter = Arc::new(RateLimiter::default());
    let ws_clients = DashMap::new();
    let servers = sqlx::query!(
        r#"SELECT id,ip,hosting_address , token FROM servers"#,
    )
        .fetch_all(&pool)
        .await?;
    for server in servers {
        let client = TarpcClient::<WebsiteToServerClient>::new(format!("{}:{SERVER_PORT}", server.ip), Some(server.token), connect_website_client);
        let connect_website_client = client.clone();
        tokio::spawn(async move {
            if let Err(e) = connect_website_client.connect().await {
                error!("Initial WebsiteServerClient connection failed: {:?}", e);
            }
        });
        ws_clients.insert(server.id, client);
    }

    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        pool: pool.clone(),
        routes: routes.clone(),
        rate_limiter: rate_limiter.clone(),
        server_vars,
        permissions: Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(900)) // 15 minutes
                .build(),
        ),
        github_install_cache: Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(20))
                .build(),
        ),
        ws_clients,
    };

    let mut task_director = TaskDirector::default();
    task_director.add_task(RefreshServerCsrf::new(csrf_server, 0, false));
    tokio::spawn(async move {
        task_director.run().await;
    });

    let app = Router::<AppState>::new()
        // .nest(
        //     "/assets",
        //     MemoryServe::new(load_assets!("../target/site/assets"))
        //         .enable_brotli(!cfg!(debug_assertions))
        //         .cache_control(CacheControl::Custom("public, max-age=31536000"))
        //         .into_router::<AppState>()
        // )
        // .nest(
        //     "/pkg",
        //     MemoryServe::new(load_assets!("../target/site/pkg"))
        //         .enable_brotli(!cfg!(debug_assertions))
        //         .cache_control(CacheControl::Custom("public, max-age=31536000"))
        //         .into_router::<AppState>()
        // )
        .route(
            "/api/{*wildcard}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .route("/api/github_webhook", post(github_webhook))
        .route("/api/github_post_install_callback", get(github_post_install_callback))
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        // .layer(
        //     CompressionLayer::new()
        //         .br(true)
        //         .zstd(true)
        //         .quality(CompressionLevel::Default)
        //         .compress_when(
        //             SizeAbove::new(256)
        //                 .and(NotForContentType::GRPC)
        //                 .and(NotForContentType::IMAGES)
        //                 .and(NotForContentType::SSE)
        //                 .and(NotForContentType::const_new("text/javascript"))
        //                 .and(NotForContentType::const_new("application/wasm"))
        //                 .and(NotForContentType::const_new("text/css")),
        //         ),
        // )
       
        .layer(
            AuthSessionLayer::<User, UserId, SessionPgPool, PgPool>::new(Some(pool.clone()))
                .with_config(auth_config),
        )
        .layer(SessionLayer::new(session_store))
        // .route_layer(axum::middleware::from_fn_with_state(
        //     rate_limiter.clone(),
        //     rate_limit_middleware,
        // ))
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
