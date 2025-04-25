use crate::app::components::csrf_field::CSRFField;
use leptos::form::ActionForm;

use leptos::prelude::{signal, Effect, ElementChild, Get, ServerFnError, Set};
use leptos::prelude::{ClassAttribute, IntoMaybeErased};
use leptos::server::ServerAction;
use leptos::{component, view, IntoView};

#[component]
pub fn NewProjectPage(
    create_project_action: ServerAction<server_fns::CreateProject>,
) -> impl IntoView {

    let (new_project_result, set_new_project_result) = signal(" ".to_string());
    Effect::new(move |_| {
        create_project_action.version().get();
        match create_project_action.value().get() {
            Some(Ok(_)) => set_new_project_result.set(String::from("Project created")),
            Some(Err(ServerFnError::ServerError(e))) => set_new_project_result.set(e.to_string()),
            _ => (),
        };
    });
    view! {
        // Separator before New Project section
        <div class="section-border">
            <h2 class="section-title">"New Project"</h2>
            <p class="section-desc">"Create a new project."</p>

            <ActionForm action=create_project_action>
                <CSRFField />

                <div class="mt-10 grid grid-cols-1 gap-x-6 gap-y-8 sm:grid-cols-6">
                    <div class="sm:col-span-4">
                        <label for="name" class="form-label">
                            "Project Name"
                        </label>
                        <div class="mt-2">
                            <input type="text" name="name" required class="form-input" />
                        </div>
                    </div>
                </div>

                <div class="mt-6 flex items-center justify-end gap-x-6">
                    <button type="submit" class="btn btn-primary">
                        "Create Project"
                    </button>
                </div>
                <div>{new_project_result}</div>
            </ActionForm>

        </div>
    }
}

pub mod server_fns {
    use leptos::prelude::ServerFnError;
    use leptos::server;
    use serde::{Deserialize, Serialize};
    use validator::Validate;
    use crate::security::utils::SANITIZED_REGEX;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::security::utils::ssr::verify_easy_hash;
        use leptos::leptos_dom::log;
    }}
    
    #[derive(Debug, Clone, Serialize, Deserialize, Validate)]
    pub struct NewProjectForm {
        #[validate(length(min = 2, max = 30), regex(path = *SANITIZED_REGEX, message="Project must contain only letters (a-z, A-Z), number (0-9) and underscores (_)"))]
        pub name: String,
    }

    #[server]
    pub async fn create_project(csrf: String, name: String) -> Result<(), ServerFnError> {
        let new_project_form = NewProjectForm{
            name: name.clone(),
        };
        new_project_form.validate()?;
        let auth = crate::ssr::auth(false)?;
        let server_vars = crate::ssr::server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;
        let user_slug = crate::security::utils::ssr::get_auth_session_user_slug(&auth).unwrap();
        match ssr::create_project(user_slug, name).await {
            Ok(project) => {
                log!("Project created: {:?}", project);
                leptos_axum::redirect(
                    format!("/user/projects/{}", project.get_slug().to_str()).as_str(),
                );
            }
            Err(e) => {
                log!("Error creating default project: {:?}", e);
            }
        }

        Ok(())
    }

    #[cfg(feature = "ssr")]
    pub mod ssr {
        use crate::models::Project;
        use common::permission::Permission;
        use common::server_action::user_action::UserAction;
        use common::{Slug, UserSlug};
        use leptos::prelude::ServerFnError;

        pub async fn create_project(
            user_slug: UserSlug,
            name: String,
        ) -> Result<Project, ServerFnError> {
            use crate::api::ssr::request_server_action;
            let pool = crate::ssr::pool()?;
            let project =
                sqlx::query!("INSERT INTO projects (name) VALUES ($1) returning id", name)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            sqlx::query!(
                "INSERT INTO permissions (user_id, project_id, permission) VALUES ($1, $2, $3)",
                user_slug.id,
                project.id,
                Permission::Owner as Permission
            )
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            request_server_action(
                UserAction::AddProject {
                    user_slug,
                    project_slug: Slug::new(project.id, name.clone()),
                }
                .into(),
            )
            .await?;
            Ok(Project {
                id: project.id,
                name,
                ..Default::default()
            })
        }
    }
}
