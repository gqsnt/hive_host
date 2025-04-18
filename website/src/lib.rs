use http::StatusCode;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

pub mod app;
pub mod error_template;
pub mod projects;
pub mod rate_limiter;
pub mod tasks;
pub mod api;
pub mod security;
pub mod models;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

#[derive(Debug, Clone, Error)]
pub enum AppError {
    #[error("Not Found")]
    NotFound,
    #[error("Internal Server Error")]
    InternalServerError,
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
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
    use common::permission::Permission;
    use crate::rate_limiter::ssr::RateLimiter;
    use axum::{
        body::Body as AxumBody,
        extract::{FromRef, Path, State},
        http::Request,
        response::{IntoResponse, Response}
    };
    use leptos::context::{provide_context, use_context};
    use leptos::logging::log;
    use leptos_axum::{handle_server_fns_with_context, AxumRouteListing};
    use moka::future::Cache;
    use portable_atomic::AtomicU128;
    use sqlx::types::Uuid;
    use sqlx::PgPool;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use leptos::config::LeptosOptions;
    use leptos::prelude::ServerFnError;
    use secrecy::SecretString;
    use common::{ProjectId, UserId};
    use crate::security::ssr::AppAuthSession;
    use crate::security::utils::ssr::stringify_u128_base64;

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
        pub token_action_auth:SecretString,
    }

    impl ServerVars{
        pub fn new(token_action_auth:SecretString) -> ServerVars {
            Self{
                csrf_server: Arc::new(CsrfServer::default()),
                token_action_auth,
            }
        }
    }


    #[derive(Debug)]
    pub struct CsrfServer(AtomicU128);

    impl Default for CsrfServer{
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

    pub fn pool() -> Result<PgPool, ServerFnError> {
        use_context::<PgPool>().ok_or_else(|| ServerFnError::ServerError("Pool missing.".into()))
    }

    pub fn server_vars() -> Result<ServerVars, ServerFnError> {
        use_context::<ServerVars>()
            .ok_or_else(|| ServerFnError::ServerError("Server vars missing.".into()))
    }

    pub fn rate_limiters() -> Result<Arc<RateLimiter>, ServerFnError> {
        use_context::<Arc<RateLimiter>>()
            .ok_or_else(|| ServerFnError::ServerError("Rate limiter missing.".into()))
    }

    pub fn auth(guest_allowed: bool) -> Result<AppAuthSession, ServerFnError> {
        let auth = use_context::<AppAuthSession>()
            .ok_or_else(|| ServerFnError::ServerError("Auth session missing.".into()));
        let is_guest = auth.as_ref().map(|auth| auth.is_anonymous()).unwrap_or_default();
        if !guest_allowed && is_guest {
            leptos_axum::redirect("/login");
            Err(ServerFnError::ServerError("Guest user not allowed.".into()))
        } else {
            auth
        }
    }

    pub fn permissions() -> Result<Permissions, ServerFnError> {
        use_context::<Permissions>()
            .ok_or_else(|| ServerFnError::ServerError("Permissions missing.".into()))
    }

    pub async fn server_fn_handler(
        State(app_state): State<AppState>,
        auth_session: AppAuthSession,
        path: Path<String>,
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
