#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::security::ssr::AppAuthSession;
    use blake2::{Blake2s256, Digest};
    use common::{UserId, UserSlug};
    use http::header::CONTENT_TYPE;
    use http::HeaderValue;
    use leptos::prelude::{use_context, ServerFnError};
    use secrecy::{ExposeSecret, SecretString};
    use uuid::Uuid;

    pub fn get_auth_session_user_id(auth_session: &AppAuthSession) -> Option<UserId> {
        auth_session.current_user.as_ref().map(|u| u.id)
    }
    pub fn get_auth_session_user_slug(auth_session: &AppAuthSession) -> Option<UserSlug> {
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
    ) -> Result<(), ServerFnError> {
        if expected_result.eq(&gen_easy_hash(input1, input2)) {
            Ok(())
        } else {
            Err(ServerFnError::ServerError(
                "Csrf does not match".to_string(),
            ))
        }
    }

    pub fn set_headers() {
        let response = match use_context::<leptos_axum::ResponseOptions>() {
            Some(ro) => ro,
            None => return, // building routes in main.rs
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
