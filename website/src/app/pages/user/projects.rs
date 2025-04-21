use crate::app::components::select::FormSelect;

use leptos::prelude::{signal};
use leptos::prelude::{AddAnyAttr, Callback, Effect, For, ServerFnError};
use leptos::prelude::{Get};

use crate::app::pages::user::projects::new_project::CreateProject;
use crate::models::Project;
use leptos::either::Either;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::{ClassAttribute, Resource, Suspend, Suspense};
use leptos::server::ServerAction;
use leptos::{component, server, view, IntoView};
use leptos_router::components::{Outlet, A};
use leptos_router::hooks::{use_location};

pub mod new_project;
pub mod project;

#[component]
pub fn ProjectsPage(create_project_action: ServerAction<CreateProject>) -> impl IntoView {
    let projects = Resource::new(
        move || create_project_action.version().get(),
        move |_| get_projects(),
    );

    let projects = move || {
        projects
            .get()
            .map(|p| p.unwrap_or_default())
            .unwrap_or_default()
    };

    let get_project_slug = || {
        let location = use_location().pathname.get().clone();
        let split = location.split("/").collect::<Vec<_>>();
        // find projets string and return next or "0"
        let mut found = false;
        for s in split.iter() {
            if found {
                return s.to_string();
            }
            if s.eq(&"projects") {
                found = true;
            }
        }
        "0".to_string()
    };
    let (project_id, set_project_id) = signal(get_project_slug());

    let handle_select_project = move |value: String| {
        let navigate = leptos_router::hooks::use_navigate();
        set_project_id(value.clone());
        match value.as_str() {
            "0" => navigate("/user/projects", Default::default()),
            _ => navigate(
                format!("/user/projects/{value}").as_str(),
                Default::default(),
            ),
        };
    };
    Effect::new(move || {
        let project_id = get_project_slug();
        set_project_id(project_id.clone());
    });
    let select_project_callback = Callback::new(move |e| {
        handle_select_project(e);
    });

    view! {
        <div>
            <h2>"Projects"</h2>
            <div class="mt-2 mb-6 flex items-center content-center space-x-2 border-b border-white/10 pb-6">
                <FormSelect name="project_id".to_string() on_change=select_project_callback>
                    <option value="0">"New Project"</option>
                    <Suspense fallback=|| {
                        view! { <option value="0">"No Projects Found"</option> }
                    }>
                        {move || Suspend::new(async move {
                            let projects = projects();
                             if projects.is_empty() {
                                Either::Right(
                                    view! { <option value="0">"No Projects Found"</option> },
                                )
                            } else {
                                Either::Left(
                                    view! {
                                        <For
                                            each=move || projects.clone()
                                            key=|project| project.id
                                            children=move |project| {
                                                let project_slug = project.get_slug().to_str();
                                                view! {
                                                    <option
                                                        value=project_slug.clone()
                                                        selected=move || project_id.get() == project_slug
                                                    >
                                                        {project.name}
                                                    </option>
                                                }
                                            }
                                        />
                                    },
                                )
                            }
                        })}

                    </Suspense>

                </FormSelect>

                <A
                    attr:class=" rounded-md bg-indigo-500 p-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-400 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500"
                    href="/user/projects"
                    on:click=move |_| {
                        set_project_id("0".to_string());
                    }
                >
                    New Project
                </A>
            </div>
            <Outlet />
        </div>
    }
}

#[server]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    use crate::security::utils::ssr::get_auth_session_user_id;

    let pool = crate::ssr::pool()?;
    let auth = crate::ssr::auth(false)?;
    let projects = sqlx::query_as!(Project,
        "SELECT * FROM projects WHERE id IN (SELECT project_id FROM permissions WHERE user_id = $1)",
        get_auth_session_user_id(&auth).unwrap()
    )
        .fetch_all(&pool)
        .await.
        map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(projects)
}
