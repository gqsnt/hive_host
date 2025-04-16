use leptos::logging::log;
use crate::BoolInput;
use leptos::prelude::{ServerFnError};
use leptos::server;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use common::server_action::user_action::UserAction;
use common::{Slug, UserId, UserSlug};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Default)]
pub enum RoleType {
    #[default]
    User,
    Admin,
}

#[server]
pub async fn get_user() -> Result<Option<User>, ServerFnError> {
    let auth = crate::ssr::auth(true)?;
    Ok(auth.current_user)
}

#[server(Login, "/api")]
pub async fn login(
    csrf: String,
    email: String,
    password: String,
    remember: Option<BoolInput>,
) -> Result<User, ServerFnError> {
    use secrecy::ExposeSecret;
    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    ssr::verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;

    let remember = remember.unwrap_or_default().into();
    let password = secrecy::SecretString::from(password.as_str());
    let pool = crate::ssr::pool()?;
    let (user, password_hash) = User::get_from_email_with_password(&email, &pool)
        .await
        .ok_or(ServerFnError::new("User not found"))?;
    match password_auth::verify_password(
        password.expose_secret().as_bytes(),
        password_hash.expose_secret(),
    ) {
        Ok(_) => {
            auth.login_user(user.id);
            auth.remember_user(remember);
            leptos_axum::redirect("/user");
            Ok(user)
        }
        Err(_) => Err(ServerFnError::ServerError(
            "Password does not match".to_string(),
        )),
    }
}

#[server(Signup, "/api")]
pub async fn signup(
    csrf: String,
    email: String,
    username: String,
    password: String,
    password_confirmation: String,
    remember: Option<BoolInput>,
) -> Result<User, ServerFnError> {
    use secrecy::ExposeSecret;
    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    ssr::verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;

    let remember = remember.unwrap_or_default().into();
    let password = secrecy::SecretString::from(password.as_str());
    let password_confirm = secrecy::SecretString::from(password_confirmation.as_str());
    let pool = crate::ssr::pool()?;

    if password.expose_secret() != password_confirm.expose_secret() {
        return Err(ServerFnError::ServerError(
            "Passwords do not match".to_string(),
        ));
    }
    let user = User::get_from_email_with_password(&email, &pool).await;
    if user.is_some() {
        return Err(ServerFnError::ServerError(
            "User already exists".to_string(),
        ));
    }
    let user = sqlx::query!(
        r#"INSERT INTO users (email, password, role, username) VALUES ($1, $2, $3, $4) returning id"#,
        email,
        password_auth::generate_hash(&password.expose_secret().as_bytes()),
        ssr::SqlRoleType::default() as ssr::SqlRoleType,
        username,
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| ServerFnError::new("Failed to create user"))?;
    auth.login_user(user.id);
    auth.remember_user(remember);
    leptos_axum::redirect("/user");
    let user= User {
        id: user.id,
        email,
        role_type: RoleType::default(),
        username,
    };
    let user_slug = user.get_slug();
    if let Err(e) = crate::permission::ssr::request_server_action(UserAction::Create {
        user_slug:user_slug.clone(),
    }.into()).await{
        log!("Error creating user: {:?}", e);
    };
    if let Err(e) = crate::projects::ssr::create_project(user_slug, "Default".to_string()).await{
        log!("Error creating default project: {:?}", e);
    };
    Ok(user)
}



#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let auth = crate::ssr::auth(false)?;
    auth.logout_user();
    leptos_axum::redirect("/");
    Ok(())
}




#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub role_type: RoleType,
    pub username:String,
}

impl User{
    pub fn get_slug(&self) -> UserSlug {
        Slug::new(self.id, self.username.clone())
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for User {}

impl Default for User {
    fn default() -> Self {
        Self {
            id: -1,
            email: "guest@mail.com".to_string(),
            role_type: RoleType::default(),
            username: "guest".to_string(),
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use anyhow::Error;
    use async_trait::async_trait;
    use crate::auth::{RoleType, User};
    use axum_session_auth::Authentication;
    use axum_session_sqlx::SessionPgPool;
    use blake2::{Blake2s256, Digest};
    use http::header::CONTENT_TYPE;
    use http::HeaderValue;
    use leptos::prelude::{use_context, ServerFnError};
    use secrecy::{ExposeSecret, SecretString};
    use serde::{Deserialize, Serialize};
    use sqlx::types::Uuid;
    use sqlx::PgPool;
    use common::UserId;

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        Ord,
        PartialOrd,
        Serialize,
        Deserialize,
        Default,
        sqlx::Type,
    )]
    #[sqlx(type_name = "role_type", rename_all = "lowercase")]
    pub enum SqlRoleType {
        Admin,
        #[default]
        User,
    }

    impl From<SqlRoleType> for RoleType {
        fn from(value: SqlRoleType) -> Self {
            match value {
                SqlRoleType::Admin => RoleType::Admin,
                SqlRoleType::User => RoleType::User,
            }
        }
    }

    pub type AppAuthSession = axum_session_auth::AuthSession<User, UserId, SessionPgPool, PgPool>;

    #[async_trait]
    impl Authentication<User, UserId, PgPool> for User {
        async fn load_user(userid: UserId, pool: Option<&PgPool>) -> Result<User, Error> {
            let pool = pool.unwrap();
            User::get_from_id(userid, pool)
                .await
                .ok_or_else(|| anyhow::anyhow!("User not found"))
        }

        fn is_authenticated(&self) -> bool {
            !self.is_anonymous()
        }

        fn is_active(&self) -> bool {
            true
        }

        fn is_anonymous(&self) -> bool {
            self.id == -1
        }
    }

    impl User {
        pub async fn get_from_id(id: UserId, pool: &PgPool) -> Option<Self> {
            let user = sqlx::query_as!(
                SqlUserShort,
                r#"SELECT id, email, role as "role: SqlRoleType",username FROM users WHERE id = $1"#,
                id
            )
            .fetch_one(pool)
            .await
            .ok()?;
            Some(Self {
                id: user.id,
                email: user.email,
                role_type: user.role.into(),
                username: user.username,
            })
        }

        pub async fn get_from_email_with_password(
            email: &str,
            pool: &PgPool,
        ) -> Option<(Self, SecretString)> {
            let user = sqlx::query_as!(
                SqlUserLong,
                r#"SELECT id, email, password, role as "role: SqlRoleType", username FROM users WHERE email = $1"#,
                email
            )
                .fetch_one(pool)
                .await.ok()?;
            Some((
                Self {
                    id: user.id,
                    email: user.email,
                    role_type: user.role.into(),
                    username: user.username,
                },
                user.password,
            ))
        }
    }

    pub fn get_auth_session_user_id(auth_session: &AppAuthSession) -> Option<UserId> {
        auth_session.current_user.as_ref().map(|u| u.id)
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
            axum::http::header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        );
        response.insert_header(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        );
        response.insert_header(
            axum::http::header::CACHE_CONTROL,
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
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        )
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlUserLong {
        pub id: UserId,
        pub email: String,
        pub password: SecretString,
        pub role: SqlRoleType,
        pub username: String,
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlUserShort {
        pub id: UserId,
        pub email: String,
        pub role: SqlRoleType,
        pub username: String,
    }
}
