#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::security::ssr::AppAuthSession;
    use crate::{AppError, AppResult};
    use blake2::{Blake2s256, Digest};
    use common::{Slug, UserId};
    use http::header::CONTENT_TYPE;
    use http::HeaderValue;
    use leptos::prelude::use_context;
    use regex::Regex;
    use secrecy::{ExposeSecret, SecretString};
    use sqlx::PgPool;
    use std::borrow::Cow;
    use std::sync::LazyLock;
    use tokio::runtime::Handle;
    use uuid::Uuid;
    use validator::{Validate, ValidationError};

    pub fn validate_password_strength(value: &str) -> Result<(), ValidationError> {
        let mut has_lowercase = false;
        let mut has_uppercase = false;
        let mut has_digit = false;
        let mut has_symbol = false;

        for c in value.chars() {
            if c.is_lowercase() {
                has_lowercase = true;
            } else if c.is_uppercase() {
                has_uppercase = true;
            } else if c.is_ascii_digit() {
                has_digit = true;
            } else {
                if !c.is_whitespace() && !c.is_control() {
                    has_symbol = true;
                }
            }
        }
        let mut password_strength_errors = vec![];

        if !has_lowercase {
            password_strength_errors.push(String::from("lowercase letter"));
        }
        if !has_uppercase {
            password_strength_errors.push(String::from("uppercase letter"));
        }
        if !has_digit {
            password_strength_errors.push(String::from("digit"));
        }
        if !has_symbol {
            password_strength_errors.push(String::from("symbol"));
        }
        if !password_strength_errors.is_empty() {
            let error_message = format!(
                "Password must contain at least one {}.",
                password_strength_errors.join(", ")
            );
            Err(ValidationError::new("password_strength").with_message(Cow::from(error_message)))
        } else {
            Ok(())
        }
    }

    #[derive(Debug, Clone, Validate)]
    pub struct PasswordForm {
        #[validate(
            // length(min = 12, max = 30),
            // custom(function="validate_password_strength")
        )]
        pub password: String,
        #[validate(must_match(other = "password"))]
        pub password_confirmation: String,
    }

    pub static SANITIZED_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[a-zA-Z0-9]+$").unwrap());

    pub struct AsyncValidationContext {
        pub pg_pool: PgPool,
        pub handle: Handle,
    }

    pub fn get_auth_session_user_id(auth_session: &AppAuthSession) -> Option<UserId> {
        auth_session.current_user.as_ref().map(|u| u.id)
    }
    pub fn get_auth_session_user_slug(auth_session: &AppAuthSession) -> Option<Slug> {
        auth_session.current_user.as_ref().map(|u| u.get_slug())
    }

    pub fn gen_128bit_base64() -> String {
        // this will issue a CSPRNG created 128 bits of entropy in base 64
        // This function only generates the CSPRNG value.
        //
        // For session cookies alternate implementations would deliver AES encrypted
        // data to the user to prevent addtional DB load on each API request including
        // the session cookie.
        //
        // for now we will only use the full random ID and hit the database with each request
        // this is an easy place to improve performance later if it is needed with high DB load
        const CUSTOM_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );
        base64::Engine::encode(&CUSTOM_ENGINE, Uuid::new_v4().as_bytes())
    }

    pub fn stringify_u128_base64(input: u128) -> String {
        const CUSTOM_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );

        base64::Engine::encode(&CUSTOM_ENGINE, input.to_be_bytes())
    }

    pub fn gen_easy_hash(input1: String, input2: SecretString) -> String {
        // forever TODO: watch for updates regarding blake2
        // <https://github.com/RustCrypto/hashes#supported-algorithms>
        let mut hasher = Blake2s256::new();
        hasher.update(format!("{input1}!{}", input2.expose_secret()).as_bytes());
        let res = hasher.finalize();
        const CUSTOM_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );
        base64::Engine::encode(&CUSTOM_ENGINE, res)
    }

    pub fn verify_easy_hash(
        input1: String,
        input2: SecretString,
        expected_result: String,
    ) -> AppResult<()> {
        expected_result
            .eq(&gen_easy_hash(input1, input2))
            .then_some(())
            .ok_or(AppError::InvalidCsrf)
    }

    pub fn set_headers() {
        let response = match use_context::<leptos_axum::ResponseOptions>() {
            Some(ro) => ro,
            None => return, // building routes in main_mini_http
        };

        //let _nonce = use_nonce().expect("a nonce to be made");

        //TODO remove after leptos sets any of these by default
        response.insert_header(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
        );
        response.insert_header(
            http::header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        );
        response.insert_header(
            http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        );
        response.insert_header(
            http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache, private"),
        );

        // CSP en mode DEV: script-src plus permissif pour le hot-reload
        // si on utilise des WebSockets sur localhost:3001
        // #[cfg(debug_assertions)]
        // {
        //     // version "rapide" (moins sécurisée):
        //     // - 'unsafe-inline' + 'unsafe-eval' => nécessaire pour le dev Leptos
        //     // - connect-src => autorise ws:// (hot reload)
        //     let csp_dev = format!(
        //         "default-src 'self'; \
        //      script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
        //      connect-src 'self' ws://127.0.0.1:3001 ws://localhost:3001; \
        //      style-src 'self' 'unsafe-inline'; \
        //      img-src 'self' data:; \
        //      object-src 'none'; \
        //      base-uri 'self'; \
        //      frame-ancestors 'none';"
        //     );
        //     response.insert_header(
        //         axum::http::header::CONTENT_SECURITY_POLICY,
        //         HeaderValue::from_str(&csp_dev).expect("invalid CSP header"),
        //     );
        // }
        //
        // // CSP en mode PROD: plus stricte, pas de ws
        // #[cfg(not(debug_assertions))]
        // {
        //     // version stricte sans websocket ni inline/unsafe-eval
        //     // EXEMPLE si tu n’as pas besoin de script dynamique ou SSR injection via nonce,
        //     // tu peux rester sur 'self' et bloquer le inline.
        //     let csp_prod = format!(
        //         "default-src 'self'; \
        //      script-src 'self'; \
        //      connect-src 'self'; \
        //      style-src 'self'; \
        //      img-src 'self'; \
        //      object-src 'none'; \
        //      base-uri 'self'; \
        //      frame-ancestors 'none';"
        //     );
        //     response.insert_header(
        //         axum::http::header::CONTENT_SECURITY_POLICY,
        //         HeaderValue::from_str(&csp_prod).expect("invalid CSP header"),
        //     );
        // }
        response.insert_header(
            http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        )
    }
}
