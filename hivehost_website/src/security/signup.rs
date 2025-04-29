
use crate::models::User;
use crate::{AppResult, BoolInput};
use leptos::server;


#[server(Signup, "/api")]
pub async fn signup(
    csrf: String,
    email: String,
    username: String,
    password: String,
    password_confirmation: String,
    remember: Option<BoolInput>,
) -> AppResult<User> {
    use crate::models::RoleType;
    use crate::security::utils::ssr::verify_easy_hash;
    use common::server_action::user_action::UserAction;
    use crate::app::pages::user::projects::new_project::server_fns::ssr::create_project;
    use secrecy::ExposeSecret;
    use crate::security::utils::ssr::AsyncValidationContext;
    use crate::security::utils::{PasswordForm, SANITIZED_REGEX};
    use tokio::runtime::Handle;
    use validator::{Validate, ValidateArgs, ValidationError};
    use crate::AppError;
    use common::{Slug};


    pub fn unique_email(email: &str, context:&AsyncValidationContext) -> Result<(), ValidationError>{
        tokio::task::block_in_place(|| {
            let AsyncValidationContext { pg_pool, handle } = context;
            let result = handle.block_on(User::get_from_email_with_password(email, pg_pool));
            if result.is_ok() {
                return Err(ValidationError::new("Email already taken"));
            }
            Ok(())
        })
    }



    #[derive(Debug, Clone, Validate)]
    #[validate(context =AsyncValidationContext)]
    pub struct SignupForm {
        #[validate(email, custom(function="unique_email", use_context))]
        pub email: String,
        #[validate(length(min = 3, max = 20),regex(path=*SANITIZED_REGEX, message="Username must contain only letters (a-z, A-Z), number (0-9) and underscores (_)"))]
        pub username: String,
        #[validate(nested)]
        pub password_form: PasswordForm,
    }


    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    let pool = crate::ssr::pool()?;
    verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;

    let form = SignupForm {
        email: email.clone(),
        username: username.to_lowercase(),
        password_form: PasswordForm {
            password: password.clone(),
            password_confirmation: password_confirmation.clone(),
        },
    };
    let context = AsyncValidationContext {
        pg_pool: pool.clone(),
        handle: Handle::current(),
    };
    form.validate_with_args(
        &context
    )?;

    let remember = remember.unwrap_or_default().into();
    let password = secrecy::SecretString::from(password.as_str());
    let user = sqlx::query!(
        r#"INSERT INTO users (email, password, role, username) VALUES ($1, $2, $3, $4) returning id"#,
        form.email,
        password_auth::generate_hash(&password.expose_secret().as_bytes()),
        RoleType::default() as RoleType,
        form.username.clone(),
    )
        .fetch_one(&pool)
        .await.map_err(AppError::from)?;
    auth.login_user(user.id);
    auth.remember_user(remember);
    leptos_axum::redirect("/user");
    let user_slug = Slug::new(user.id, form.username.clone());
    let user = User {
        id: user.id,
        email,
        role_type: RoleType::default(),
        username:form.username.clone(),
        slug:user_slug.to_string(),
    };
    let user_slug = user.get_slug();
    crate::api::ssr::request_server_action(
        UserAction::Create {
            user_slug: user_slug.clone(),
        }
        .into(),
    )
    .await?;
    create_project(user_slug, "default".to_string()).await?;
    Ok(user)
}
