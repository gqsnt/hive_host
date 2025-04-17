use leptos::prelude::ServerFnError;
use leptos::server;
use serde::{Deserialize, Serialize};
use common::{Slug, UserId, UserSlug};

pub mod signup;
pub mod login;
pub mod permission;
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
    Ok(auth.current_user)
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Default)]
pub enum RoleType {
    #[default]
    User,
    Admin,
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
    use axum_session_auth::Authentication;
    use axum_session_sqlx::SessionPgPool;
    use secrecy::{SecretString};
    use serde::{Deserialize, Serialize};
    use sqlx::PgPool;
    use common::UserId;
    use crate::security::{RoleType, User};

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
