use leptos::server;
use crate::AppResult;
use crate::models::User;

pub mod login;
pub mod permission;
pub mod signup;
pub mod utils;

#[server]
pub async fn logout() -> AppResult<()>{
    let auth = crate::ssr::auth(false)?;
    auth.logout_user();
    leptos_axum::redirect("/");
    Ok(())
}

#[server]
pub async fn get_user() -> AppResult<User> {
    use crate::AppError;
    let auth = crate::ssr::auth(true)?;
    if let Some(user) = auth.current_user {
        Ok(user)
    }else{
        leptos_axum::redirect("/login");
        Err(AppError::UnauthorizedAuthAccess)
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::models::{RoleType, User};
    use crate::AppResult;
    use anyhow::Error;
    use async_trait::async_trait;
    use axum_session_auth::Authentication;
    use axum_session_sqlx::SessionPgPool;
    use common::{UserId, UserSlugStr};
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
                r#"SELECT id, role as "role: RoleType",username, slug FROM users WHERE id = $1"#,
                id
            )
            .fetch_one(pool)
            .await
            .ok()?;
            Some(Self {
                id: user.id,
                role_type: user.role,
                username: user.username,
                slug: user.slug,
            })
        }
        
        pub async fn exist(
            email: &str,
            pool: &PgPool,
        ) -> AppResult<bool> {
            let user = sqlx::query!(
                r#"SELECT  email FROM users WHERE email = $1"#,
                email
            )
                .fetch_optional(pool)
                .await?;
            Ok(user.is_some())
        }
        
        

        pub async fn get_id_password(
            email: &str,
            pool: &PgPool,
        ) -> AppResult<(UserId, SecretString)> {
            let user = sqlx::query!(
                r#"SELECT  id, email, password FROM users WHERE email = $1"#,
                email
            )
                .fetch_one(pool)
                .await?;
            Ok((user.id, SecretString::from(user.password)))
        }
    }
    
    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlUserShort {
        pub id: UserId,
        pub role: RoleType,
        pub username: String,
        slug : UserSlugStr,
    }
}
