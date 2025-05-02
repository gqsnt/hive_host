
use crate::{AppResult, BoolInput};
use leptos::server;


#[server(Login, "/api")]
pub async fn login(
    csrf: String,
    email: String,
    password: String,
    remember: Option<BoolInput>,
) -> AppResult<()> {
    use crate::models::User;
    use crate::security::utils::ssr::verify_easy_hash;
    use secrecy::ExposeSecret;
    use crate::AppError;

    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;

    let remember = remember.unwrap_or_default().into();
    let password = secrecy::SecretString::from(password.as_str());
    let pool = crate::ssr::pool()?;

    let (user_id, password_hash) = User::get_id_password(&email, &pool)
        .await
        .map_err(|_| AppError::InvalidCredentials)?;
    password_auth::verify_password(
        password.expose_secret().as_bytes(),
        password_hash.expose_secret(),
    )
    .map_err(|_| AppError::InvalidCredentials)?;
    auth.login_user(user_id);
    auth.remember_user(remember);
    leptos_axum::redirect("/user");
    Ok(())
}
