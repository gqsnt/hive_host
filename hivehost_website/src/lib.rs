use std::str::FromStr;
use common::ParseSlugError;
use thiserror::Error;

#[cfg(feature = "ssr")]
use axum_session::SessionError;
use leptos::prelude::{FromServerFnError, ServerFnErrorErr};
use leptos::server_fn::codec::{BincodeEncoding};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::migrate::MigrateError;
#[cfg(feature = "ssr")]
use validator::{ValidationError, ValidationErrors};


pub mod api;
pub mod app;
pub mod models;
pub mod rate_limiter;
pub mod security;
pub mod tasks;


#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

pub type AppResult<T> = Result<T, AppError>;
pub type ServerFnResult<T> = Result<T, AppError>;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum AppError {
    #[cfg(feature = "ssr")]
    #[error("Reqwuest error {0}")]
    RequestError(String),
    #[cfg(feature = "ssr")]
    #[error("Sqlx error {0}")]
    SqlxError(String),
    #[cfg(feature = "ssr")]
    #[error("Migrate error {0}")]
    MigrateError(String),
    #[cfg(feature = "ssr")]
    #[error("Tarpc error {0}")]
    RpcError(String),
    #[cfg(feature = "ssr")]
    #[error("DotEnv error: {0}")]
    DotEnv(String),
    #[cfg(feature = "ssr")]
    #[error("Session error: {0}")]
    SessionError(String),
    #[cfg(feature = "ssr")]
    #[error("AddrParse error: {0}")]
    AddrParse(String),
    #[error("Pool not found")]
    PoolNotFound,
    #[error("Server vars not found")]
    ServerVarsNotFound,
    #[error("Rate limiter not found")]
    RateLimiterNotFound,
    #[error("Multiplexer client not found")]
    WebsiteToServerClientNotFound,
    #[error("Auth not found")]
    AuthNotFound,
    #[error("Permissions not found")]
    PermissionsNotFound,
    #[error("Invalid ProjectSlug")]
    InvalidProjectSlug,
    #[error("Invalid Slug {0}")]
    ParseSlug(#[from] ParseSlugError),
    #[cfg(feature = "ssr")]
    #[error("Validation Error {0}")]
    ValidationError(#[from] ValidationError),
    #[cfg(feature = "ssr")]
    #[error("Validation Errors {0}")]
    ValidationErrors(#[from] ValidationErrors),
    #[error("ServerFnError {0}")]
    ServerFnError(#[from] ServerFnErrorErr),
    #[error("Unauthorized Auth Access")]
    UnauthorizedAuthAccess,
    #[error("Unauthorized Project Access")]
    UnauthorizedProjectAccess,
    #[error("Unauthorized Project Action")]
    UnauthorizedProjectAction,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Invalid Csrf")]
    InvalidCsrf,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Custom: {0}")]
    Custom(String),
    #[error("To much Snapshots")]
    ToMuchSnapshots,
    #[error("Cant delete active snapshot")]
    CantDeleteActiveSnapshot,
    #[error("No Active snapshot")]
    NoActiveSnapshot,
    #[cfg(feature = "ssr")]
    #[error("Io error: {0}")]
    Io(String),
    #[cfg(feature = "ssr")]
    #[error("TarpcClientError: {0}")]
    TrpcClientError(#[from] common::tarpc_client::TarpcClientError)
}

#[cfg(feature = "ssr")]
macro_rules! impl_from_to_string {
    ($res:path, $from:ty) => {
        impl From<$from> for AppError {
            fn from(value: $from) -> Self {
                $res(value.to_string())
            }
        }
    };
}

#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::AddrParse, std::net::AddrParseError);
#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::SessionError, SessionError);
#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::MigrateError, MigrateError);
#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::DotEnv, dotenvy::Error);
#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::RequestError, reqwest::Error);
#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::SqlxError, sqlx::Error);
#[cfg(feature = "ssr")]
impl_from_to_string!(AppError::Io, std::io::Error);




impl FromServerFnError for AppError {
    type Encoder = BincodeEncoding;
    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        value.into()
    }
}

pub struct BoolInput(pub bool);

impl From<BoolInput> for bool {
    fn from(value: BoolInput) -> Self {
        value.0
    }
}

impl FromStr for BoolInput {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(BoolInput(true)),
            "false" => Ok(BoolInput(false)),
            _ => Err(AppError::Custom(format!("Invalid bool input: {s}"))),
        }
    }
}


#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::app::shell;
    use crate::rate_limiter::ssr::RateLimiter;
    use crate::security::ssr::AppAuthSession;
    use crate::security::utils::ssr::stringify_u128_base64;
    use crate::{AppError, AppResult};
    use axum::{
        body::Body as AxumBody,
        extract::{FromRef, Path, State},
        http::Request,
        response::{IntoResponse, Response},
    };
    use common::server_action::permission::Permission;
    use common::{ProjectId, UserId};
    use leptos::config::LeptosOptions;
    use leptos::context::{provide_context, use_context};
    use leptos_axum::{handle_server_fns_with_context, AxumRouteListing};
    use moka::future::Cache;
    use portable_atomic::AtomicU128;
    use secrecy::SecretString;
    use sqlx::types::Uuid;
    use sqlx::PgPool;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use tarpc::client;
    use tarpc::tokio_serde::formats::Bincode;
    use common::server_action::tarpc::WebsiteToServerClient;
    use common::tarpc_client::{TarpcClient, TarpcClientError};

    pub type Permissions = Arc<Cache<(UserId, ProjectId), Permission>>;
    pub type WsClient =  Arc<TarpcClient<WebsiteToServerClient>>;

    #[derive(Clone, FromRef)]
    pub struct AppState {
        pub pool: PgPool,
        pub leptos_options: LeptosOptions,
        pub routes: Vec<AxumRouteListing>,
        pub permissions: Permissions,
        pub server_vars: ServerVars,
        pub rate_limiter: Arc<RateLimiter>,
        pub ws_client:WsClient
    }

    #[derive(Debug, Clone)]
    pub struct ServerVars {
        pub csrf_server: Arc<CsrfServer>,
        pub server_url: Arc<String>,
        pub server_url_front: Arc<String>,
        pub server_addr: Arc<String>,
        pub server_addr_front: Arc<String>,
        pub hosting_url: Arc<String>,
        pub token_action_auth: SecretString,
    }

    impl ServerVars {
        pub fn new(
            token_action_auth: SecretString,
            server_url: String,
            server_url_front: String,
            server_addr: String,
            server_addr_front: String,
            hosting_url: String,
        ) -> ServerVars {
            Self {
                csrf_server: Arc::new(CsrfServer::default()),
                token_action_auth,
                server_url: Arc::new(server_url),
                server_url_front: Arc::new(server_url_front),
                server_addr: Arc::new(server_addr),
                server_addr_front: Arc::new(server_addr_front),
                hosting_url: Arc::new(hosting_url),
            }
        }
    }

    #[derive(Debug)]
    pub struct CsrfServer(AtomicU128);

    impl Default for CsrfServer {
        fn default() -> Self {
            Self(Uuid::new_v4().as_u128().into())
        }
    }

    impl CsrfServer {
        pub fn to_secret(&self) -> SecretString {
            SecretString::from(stringify_u128_base64(self.0.load(Ordering::Relaxed)))
        }

        pub fn refresh(&self) {
            self.0.store(Uuid::new_v4().as_u128(), Ordering::Relaxed);
        }
    }

    pub fn pool() -> AppResult<PgPool> {
        use_context::<PgPool>().ok_or(AppError::PoolNotFound)
    }
    pub fn ws_client() -> AppResult<WsClient> {
        use_context::<WsClient>().ok_or(AppError::WebsiteToServerClientNotFound)
    }
    

    pub fn server_vars() -> AppResult<ServerVars> {
        use_context::<ServerVars>().ok_or(AppError::ServerVarsNotFound)
    }

    pub fn rate_limiters() -> AppResult<Arc<RateLimiter>> {
        use_context::<Arc<RateLimiter>>().ok_or(AppError::RateLimiterNotFound)
    }

    pub fn auth(guest_allowed: bool) -> AppResult<AppAuthSession> {
        let auth = use_context::<AppAuthSession>().ok_or(AppError::AuthNotFound)?;
        if !guest_allowed && auth.is_anonymous() {
            leptos_axum::redirect("/login");
            Err(AppError::AuthNotFound)
        } else {
            Ok(auth)
        }
    }

    pub fn permissions() -> AppResult<Permissions> {
        use_context::<Permissions>().ok_or(AppError::PermissionsNotFound)
    }

    pub async fn server_fn_handler(
        State(app_state): State<AppState>,
        auth_session: AppAuthSession,
        _path: Path<String>,
        request: Request<AxumBody>,
    ) -> impl IntoResponse {
        handle_server_fns_with_context(
            move || {
                provide_context(auth_session.clone());
                provide_context(app_state.permissions.clone());
                provide_context(app_state.pool.clone());
                provide_context(app_state.server_vars.clone());
                provide_context(app_state.rate_limiter.clone());
                provide_context(app_state.ws_client.clone());
            },
            request,
        )
        .await
    }

    pub async fn leptos_routes_handler(
        auth_session: AppAuthSession,
        state: State<AppState>,
        req: Request<AxumBody>,
    ) -> Response {
        let State(app_state) = state.clone();
        let options = app_state.leptos_options.clone();
        let handler = leptos_axum::render_route_with_context(
            app_state.routes.clone(),
            move || {
                provide_context(auth_session.clone());
                provide_context(app_state.permissions.clone());
                provide_context(app_state.pool.clone());
                provide_context(app_state.server_vars.clone());
                provide_context(app_state.ws_client.clone());
                
            },
            move || shell(app_state.leptos_options.clone()),
        );
        handler(State(options), req).await.into_response()
    }


    pub async fn connect_website_client(addr: String) -> Result<WebsiteToServerClient, TarpcClientError> {
        let mut transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default);
        transport
            .config_mut()
            .max_frame_length(usize::MAX);
        Ok(WebsiteToServerClient::new(client::Config::default(), transport.await?).spawn())
        
    }
}
