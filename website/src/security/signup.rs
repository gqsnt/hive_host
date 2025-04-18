use leptos::prelude::ServerFnError;
use leptos::server;
use crate::BoolInput;
use crate::models::User;
#[server(Signup, "/api")]
pub async fn signup(
    csrf: String,
    email: String,
    username: String,
    password: String,
    password_confirmation: String,
    remember: Option<BoolInput>,
) -> Result<User, ServerFnError> {
    use crate::models::RoleType;
    use secrecy::ExposeSecret;
    use common::server_action::user_action::UserAction;
    use leptos::logging::log;
    use crate::security::ssr::SqlRoleType;
    use crate::security::utils::ssr::verify_easy_hash;

    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    verify_easy_hash(
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
        SqlRoleType::default() as SqlRoleType,
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
    if let Err(e) = crate::api::ssr::request_server_action(UserAction::Create {
        user_slug:user_slug.clone(),
    }.into()).await{
        log!("Error creating user: {:?}", e);
    };
    if let Err(e) = crate::projects::ssr::create_project(user_slug, "Default".to_string()).await{
        log!("Error creating default project: {:?}", e);
    };
    Ok(user)
}