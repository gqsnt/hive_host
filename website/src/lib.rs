use common::SlugParseError;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

#[cfg(feature = "ssr")]
use axum_session::SessionError;
use leptos::prelude::{FromServerFnError, ServerFnErrorErr};
use leptos::server_fn::codec::JsonEncoding;
#[cfg(feature = "ssr")]
use sqlx::migrate::MigrateError;
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




#[derive(Debug, Error, Deserialize, Serialize, Clone)]
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
    #[error("DotEnv error: {0}")]
    DotEnv(String),
    #[cfg(feature = "ssr")]
    #[error("Session error: {0}")]
    SessionError(String ),
    #[cfg(feature = "ssr")]
    #[error("AddrParse error: {0}")]
    AddrParse(String),
    #[error("gloo_net error: {0}")]
    GlooNet(String),
    #[error("Pool not found")]
    PoolNotFound,
    #[error("Server vars not found")]
    ServerVarsNotFound,
    #[error("Rate limiter not found")]
    RateLimiterNotFound,
    #[error("Auth not found")]
    AuthNotFound,
    #[error("Permissions not found")]
    PermissionsNotFound,
    #[error("Invalid ProjectSlug")]
    InvalidProjectSlug,
    #[error("Invalid Slug {0}")]
    ParseSlug(#[from] SlugParseError),
    #[error("Validation Error {0}")]
    ValidationError(#[from] ValidationError),
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
}

macro_rules! impl_from_to_string {
    ($res:path, $from:ty) => {
        impl From<$from> for AppError {
            fn from(value: $from) -> Self {
                $res(value.to_string())
            }
        }
    };
}

impl_from_to_string!(AppError::GlooNet, gloo_net::Error);
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



impl FromServerFnError for AppError{
    type Encoder = JsonEncoding;
    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        value.into()
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoolInput(pub bool);

impl From<BoolInput> for bool {
    fn from(value: BoolInput) -> Self {
        value.0
    }
}

impl Serialize for BoolInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0 {
            serializer.serialize_some("on")
        } else {
            serializer.serialize_none()
        }
    }
}

impl<'de> Deserialize<'de> for BoolInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // attend une Option<String>
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(BoolInput(matches!(opt.as_deref(), Some("on"))))
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
    use common::permission::Permission;
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

    pub type Permissions = Arc<Cache<(UserId, ProjectId), Permission>>;

    #[derive(Clone, FromRef)]
    pub struct AppState {
        pub pool: PgPool,
        pub leptos_options: LeptosOptions,
        pub routes: Vec<AxumRouteListing>,
        pub permissions: Permissions,
        pub server_vars: ServerVars,
        pub rate_limiter: Arc<RateLimiter>,
    }

    #[derive(Debug, Clone)]
    pub struct ServerVars {
        pub csrf_server: Arc<CsrfServer>,
        pub server_url: Arc<String>,
        pub hosting_url: Arc<String>,
        pub token_action_auth: SecretString,
    }

    impl ServerVars {
        pub fn new(
            token_action_auth: SecretString,
            server_url: String,
            hosting_url: String,
        ) -> ServerVars {
            Self {
                csrf_server: Arc::new(CsrfServer::default()),
                token_action_auth,
                server_url: Arc::new(server_url),
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
            },
            move || shell(app_state.leptos_options.clone()),
        );
        handler(State(options), req).await.into_response()
    }
}
