use crate::app::components::select::FormSelectIcon;
use leptos::prelude::{OnAttribute, OnTargetAttribute, PropAttribute, Transition};

use leptos::prelude::signal;
use leptos::prelude::Get;
use leptos::prelude::{Effect, For};

use crate::app::pages::user::projects::new_project::server_fns::CreateProject;
use leptos::either::Either;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::{ClassAttribute, Resource, Suspend};
use leptos::server::ServerAction;
use leptos::{component, view, IntoView};
use leptos_router::components::Outlet;
use leptos_router::hooks::use_location;

pub mod new_project;
pub mod project;

#[component]
pub fn ProjectsPage(create_project_action: ServerAction<CreateProject>) -> impl IntoView {
    let projects = Resource::new(
        move || create_project_action.version().get(),
        move |_| server_fns::get_projects(),
    );

    let get_project_slug = move |path: String| {
        let split = path.split("/").collect::<Vec<_>>();
        let mut found = false;
        for s in split.iter() {
            if found {
                return Some(s.to_string());
            }
            if s.eq(&"projects") {
                found = true;
            }
        }
        None
    };
    
    let location_path = use_location().pathname.get();
    let (project_id, set_project_id) = signal(get_project_slug(location_path));

    let handle_select_project = move |value: Option<String>| {
        let navigate = leptos_router::hooks::use_navigate();
        set_project_id(value.clone());
        match value {
            None => navigate("/user/projects", Default::default()),
            Some(project_slug) => navigate(
                format!("/user/projects/{project_slug}").as_str(),
                Default::default(),
            ),
        };
    };
    Effect::new(move || {
        let location = use_location().pathname.get();
        let location_project_id = get_project_slug(location);
        if location_project_id != project_id(){
            set_project_id(location_project_id.clone());
        }
    });

    view! {
        <div>
            <h2>"Projects"</h2>
            <div class="mt-2 mb-6 flex items-center content-center space-x-2 border-b border-white/10 pb-6">
                <div class="relative">
                    <select
                        name="project_id"
                        class="form-select"
                        prop:value=move || project_id.get().unwrap_or_default()
                        on:change:target=move |e| {
                            let target_value = e.target().value();
                            if target_value.is_empty() {
                                handle_select_project(None);
                            } else {
                                handle_select_project(Some(target_value));
                            }
                        }
                    >
                        <option value="">Select a project</option>

                        <Transition fallback=move || {
                            view! { <option>Loading ...</option> }
                        }>
                            {move || Suspend::new(async move {
                                let projects = projects.await.unwrap_or_default();
                                if projects.is_empty() {
                                    Either::Right(view! { <option>Create a project</option> })
                                } else {
                                    Either::Left(
                                        view! {
                                            <For
                                                each=move || projects.clone()
                                                key=move |project| project.id
                                                children=move |project| {
                                                    view! {
                                                        <option
                                                            value=project.slug.clone()
                                                            selected=move || {
                                                                project.slug.clone() == project_id.get().unwrap_or_default()
                                                            }
                                                        >
                                                            {project.slug.clone()}
                                                        </option>
                                                    }
                                                }
                                            />
                                        },
                                    )
                                }
                            })}
                        </Transition>

                    </select>
                    <FormSelectIcon />
                </div>
                <button
                    class="btn btn-primary"
                    on:click=move |e| {
                        e.prevent_default();
                        handle_select_project(None);
                    }
                >
                    New Project
                </button>
            </div>
            <Outlet />
        </div>
    }
}

pub mod server_fns {
    use crate::models::Project;
    use crate::AppResult;
    use leptos::server;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
            use crate::security::utils::ssr::get_auth_session_user_id;

    }}

    #[server]
    pub async fn get_projects() -> AppResult<Vec<Project>> {
        let pool = crate::ssr::pool()?;
        let auth = crate::ssr::auth(false)?;
        let projects = sqlx::query_as!(Project,
        "SELECT id,name,active_snapshot_id, slug FROM projects WHERE id IN (SELECT project_id FROM permissions WHERE user_id = $1)",
        get_auth_session_user_id(&auth).unwrap()
    )
            .fetch_all(&pool)
            .await?;
        Ok(projects)
    }
}
