use crate::{AppResult};
use leptos::server;
use leptos::server_fn::codec::Bitcode;

#[server(Signup, "/api", input=Bitcode, output=Bitcode)]
pub async fn signup(
    csrf: String,
    email: String,
    username: String,
    password: String,
    password_confirmation: String,
    remember: bool,
) -> AppResult<()> {
    use crate::app::pages::user::projects::new_project::server_fns::ssr::create_project;
    use crate::models::RoleType;
    use crate::models::User;
    use crate::security::utils::ssr::verify_easy_hash;
    use crate::security::utils::ssr::AsyncValidationContext;
    use crate::security::utils::ssr::PasswordForm;
    use crate::security::utils::ssr::SANITIZED_REGEX;
    use crate::AppError;
    use common::website_to_server::server_action::user_action::ServerUserAction;
    use common::Slug;
    use leptos::logging::log;
    use secrecy::ExposeSecret;
    use tokio::runtime::Handle;
    use validator::{Validate, ValidateArgs, ValidationError};

    pub fn unique_email(
        email: &str,
        context: &AsyncValidationContext,
    ) -> Result<(), ValidationError> {
        tokio::task::block_in_place(|| {
            let AsyncValidationContext { pg_pool, handle } = context;
            let result = handle.block_on(User::exist(email, pg_pool));
            match result {
                Ok(exist) => {
                    if exist {
                        Err(ValidationError::new("Email already taken"))
                    } else {
                        Ok(())
                    }
                }
                Err(e) => {
                    log!("Error checking email uniqueness: {:?}", e);
                    Err(ValidationError::new("Database error"))
                }
            }
        })
    }

    #[derive(Debug, Clone, Validate)]
    #[validate(context =AsyncValidationContext)]
    pub struct SignupForm {
        #[validate(email, custom(function = "unique_email", use_context))]
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
    form.validate_with_args(&context)?;
    
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
    crate::api::ssr::request_server_action(
        ServerUserAction::Create {
            user_slug: user_slug.clone(),
        }
        .into(),
    )
    .await?;
    create_project(user_slug, "default".to_string()).await?;
    Ok(())
}
