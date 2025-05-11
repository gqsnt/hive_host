use crate::app::components::select::FormSelectIcon;
use leptos::prelude::{GetUntracked, OnAttribute, OnTargetAttribute, PropAttribute, Transition};

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
use leptos_router::hooks::{use_location, use_navigate};
use crate::app::pages::user::projects::project::ProjectSection;

pub mod new_project;
pub mod project;

#[component]
pub fn ProjectsPage(create_project_action: ServerAction<CreateProject>) -> impl IntoView {
    let projects = Resource::new_bincode(
        move || create_project_action.version().get(),
        move |_| server_fns::get_projects(),
    );

    let get_current_project_slug_from_path = move |path_str: &str| {
        let segments: Vec<&str> = path_str.split('/').filter(|s| !s.is_empty()).collect();
        // Expected: "user", "projects", "{slug}"
        if let Some(projects_idx) = segments.iter().position(|&seg| seg == "projects") {
            if segments.len() > projects_idx + 1 {
                return Some(segments[projects_idx + 1].to_string());
            }
        }
        None
    };

    let location_pathname = use_location().pathname;
    let (current_project_slug, set_current_project_slug) = signal(get_current_project_slug_from_path(&location_pathname.get_untracked()));

    let handle_select_project = move |new_slug_option: Option<String>| {
        let navigate = use_navigate();
        let current_path = location_pathname.get(); // Get current path before navigation
        
        let current_section_enum = {
            let segments: Vec<&str> = current_path.split('/').filter(|s| !s.is_empty()).collect();
            if let Some(projects_idx) = segments.iter().position(|&seg| seg == "projects") {
                if segments.len() > projects_idx + 2 {
                    ProjectSection::from_first_segment(segments[projects_idx + 2])
                } else {
                    ProjectSection::default()
                }
            } else {
                ProjectSection::default()
            }
        };
        
        set_current_project_slug(new_slug_option.clone());

        match new_slug_option {
            None => {
                navigate("/user/projects", Default::default());
            }
            Some(slug_value) => {
                let target_path = current_section_enum.href(&slug_value);
                navigate(&target_path, Default::default());
            }
        };
    };
    Effect::new(move |_old_path| {
        let current_path_str = location_pathname.get();
        let slug_from_url = get_current_project_slug_from_path(&current_path_str);
        if slug_from_url != current_project_slug.get() {
            set_current_project_slug(slug_from_url);
        }
        current_path_str
    });

    view! {
        <div>
            <h2>"Projects"</h2>
            <div class="mt-2 mb-6 flex items-center content-center space-x-2 border-b border-white/10 pb-6">
                <div class="relative">
                    <select
                        name="project_id"
                        class="form-select"
                        prop:value=move || current_project_slug.get().unwrap_or_default()
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
                                                                project.slug.clone()
                                                                    == current_project_slug.get().unwrap_or_default()
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
    use leptos::server_fn::codec::Bincode;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
            use crate::security::utils::ssr::get_auth_session_user_id;

    }}

    #[server(input=Bincode, output=Bincode)]
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
