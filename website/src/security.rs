use leptos::prelude::ServerFnError;
use leptos::server;

use crate::models::User;


pub mod login;
pub mod permission;
pub mod signup;
pub mod utils;

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let auth = crate::ssr::auth(false)?;
    auth.logout_user();
    leptos_axum::redirect("/");
    Ok(())
}

#[server]
pub async fn get_user() -> Result<Option<User>, ServerFnError> {
    let auth = crate::ssr::auth(true)?;
    if auth.is_anonymous() {
        leptos_axum::redirect("/login");
        return Ok(None);
    }
    Ok(auth.current_user)
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::models::{RoleType, User};
    use anyhow::Error;
    use async_trait::async_trait;
    use axum_session_auth::Authentication;
    use axum_session_sqlx::SessionPgPool;
    use common::UserId;
    use secrecy::SecretString;
    use sqlx::PgPool;

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
            !self.is_anonymous()
        }

        fn is_anonymous(&self) -> bool {
            self.id == -1
        }
    }

    impl User {
        pub async fn get_from_id(id: UserId, pool: &PgPool) -> Option<Self> {
            let user = sqlx::query_as!(
                SqlUserShort,
                r#"SELECT id, email, role as "role: RoleType",username FROM users WHERE id = $1"#,
                id
            )
            .fetch_one(pool)
            .await
            .ok()?;
            Some(Self {
                id: user.id,
                email: user.email,
                role_type: user.role,
                username: user.username,
            })
        }

        pub async fn get_from_email_with_password(
            email: &str,
            pool: &PgPool,
        ) -> Option<(Self, SecretString)> {
            let user = sqlx::query_as!(
                SqlUserLong,
                r#"SELECT id, email, password, role as "role: RoleType", username FROM users WHERE email = $1"#,
                email
            )
                .fetch_one(pool)
                .await.ok()?;
            Some((
                Self {
                    id: user.id,
                    email: user.email,
                    role_type: user.role,
                    username: user.username,
                },
                user.password,
            ))
        }
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlUserLong {
        pub id: UserId,
        pub email: String,
        pub password: SecretString,
        pub role: RoleType,
        pub username: String,
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlUserShort {
        pub id: UserId,
        pub email: String,
        pub role: RoleType,
        pub username: String,
    }
}
