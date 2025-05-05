use leptos::prelude::{expect_context, signal, Effect, ElementChild, Get, NodeRef, NodeRefAttribute, OnAttribute, Set};
use leptos::prelude::{ClassAttribute, IntoMaybeErased};
use leptos::server::ServerAction;
use leptos::{component, view, IntoView};
use leptos::html::Input;
use reactive_stores::Store;
use crate::app::pages::{GlobalState, GlobalStateStoreFields};

#[component]
pub fn NewProjectPage(
    create_project_action: ServerAction<server_fns::CreateProject>,
) -> impl IntoView {
    let global_store: Store<GlobalState> = expect_context();

    let (new_project_result, set_new_project_result) = signal(" ".to_string());
    Effect::new(move |_| {
        create_project_action.version().get();
        match create_project_action.value().get() {
            Some(Ok(_)) => set_new_project_result.set(String::from("Project created")),
            Some(Err(e)) => set_new_project_result.set(e.to_string()),
            _ => (),
        };
    });
    let project_name_ref= NodeRef::<Input>::default();
    
    let on_new_project = move |event: web_sys::SubmitEvent| {
        event.prevent_default();
        create_project_action.dispatch(server_fns::CreateProject {
            csrf: global_store.csrf().get().unwrap_or_default(),
            name: project_name_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
        });
    };
    
    view! {
        // Separator before New Project section
        <div class="section-border">
            <h2 class="section-title">"New Project"</h2>
            <p class="section-desc">"Create a new project."</p>

            <form on:submit=on_new_project>
                <div class="mt-10 grid grid-cols-1 gap-x-6 gap-y-8 sm:grid-cols-6">
                    <div class="sm:col-span-4">
                        <label for="name" class="form-label">
                            "Project Name"
                        </label>
                        <div class="mt-2">
                            <input
                                type="text"
                                node_ref=project_name_ref
                                name="name"
                                required
                                class="form-input"
                            />
                        </div>
                    </div>
                </div>

                <div class="mt-6 flex items-center justify-end gap-x-6">
                    <button type="submit" class="btn btn-primary">
                        "Create Project"
                    </button>
                </div>
                <div>{new_project_result}</div>
            </form>

        </div>
    }
}

pub mod server_fns {
    use crate::AppResult;
    use leptos::server;
    use leptos::server_fn::codec::Bincode;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::security::utils::ssr::verify_easy_hash;
        use leptos::leptos_dom::log;

    }}

    #[server(input=Bincode, output=Bincode)]
    pub async fn create_project(csrf: String, name: String) -> AppResult<()> {
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
                leptos_axum::redirect(format!("/user/projects/{}", project.get_slug()).as_str());
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
        use crate::security::utils::ssr::SANITIZED_REGEX;
        use crate::AppResult;
        use common::website_to_server::permission::Permission;
        use common::website_to_server::server_action::user_action::ServerUserAction;
        use common::Slug;
        use validator::{Validate, ValidationError};

        pub fn parse_repo(repo_url: &str) -> Result<(String, String, String), ValidationError> {
            if repo_url.is_empty() {
                return Err(ValidationError::new("invalid_git_repo"));
            };
            let https_git = "https://github.com/";
            let ssh_git = "git@github.com:";

            let (before, after) = if repo_url.starts_with(https_git) {
                (https_git.to_string(), repo_url.replace(https_git, ""))
            } else if repo_url.starts_with(ssh_git) {
                (ssh_git.to_string(), repo_url.replace(ssh_git, ""))
            } else {
                return Err(ValidationError::new("invalid_git_repo"));
            };
            let (user_name, repo_name) = after
                .split_once("/")
                .ok_or_else(|| ValidationError::new("invalid_git_repo"))?;

            Ok((before, user_name.to_string(), repo_name.to_string()))
        }

        pub fn validate_git_repo(repo_url: &str) -> Result<(), ValidationError> {
            parse_repo(repo_url)?;
            Ok(())
        }

        #[derive(Debug, Clone, Validate)]
        pub struct CreateProjectForm {
            #[validate(length(min = 2, max = 30), regex(path = *SANITIZED_REGEX, message="Project must contain only letters (a-z, A-Z), number (0-9) and underscores (_)"))]
            pub name: String,
        }

        pub async fn create_project(user_slug: Slug, name: String) -> AppResult<Project> {
            use crate::api::ssr::request_server_action;
            let pool = crate::ssr::pool()?;
            let project_form = CreateProjectForm { name: name.clone() };
            project_form.validate()?;

            let project = sqlx::query!(
                "INSERT INTO projects (name) VALUES ($1) returning id",
                project_form.name
            )
            .fetch_one(&pool)
            .await?;
            sqlx::query!(
                "INSERT INTO permissions (user_id, project_id, permission) VALUES ($1, $2, $3)",
                user_slug.id,
                project.id,
                Permission::Owner as Permission
            )
            .execute(&pool)
            .await?;
            request_server_action(
                ServerUserAction::AddProject {
                    user_slug,
                    project_slug: Slug::new(project.id, project_form.name),
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
