use leptos::prelude::{expect_context, OnceResource, Read, Signal, Update};
use leptos::prelude::{AddAnyAttr, Resource, Suspend, Suspense};
use std::fmt::Display;
pub mod project_dashboard;
pub mod project_files;
pub mod project_settings;
pub mod project_team;

use leptos::context::provide_context;
use leptos::prelude::{
    signal, ClassAttribute, CollectView, Effect, Get, Memo, ReadSignal, Set, WriteSignal,
};
use leptos::{component, view, IntoView, Params};
use leptos::logging::log;
use leptos_router::hooks::{use_location, use_params};

use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::IntoMaybeErased;
use leptos_router::components::{Outlet, A};
use leptos_router::params::{Params, ParamsError};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use crate::app::components::csrf_field::generate_csrf;
use crate::app::{get_hosting_url, get_server_url};
use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use crate::app::pages::user::projects::project::server_fns::get_project;

#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectParams {
    pub project_slug: String,
}

pub type MemoProjectParams = Memo<Result<ProjectParams, ParamsError>>;

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug, EnumIter)]
pub enum ProjectSection {
    #[default]
    Dashboard,
    Team,
    Files,
    Settings,
}
impl ProjectSection {
    pub fn href(&self, base: &str) -> String {
        match self {
            ProjectSection::Dashboard => format!("/user/projects/{base}"),
            ProjectSection::Team => format!("/user/projects/{base}/team"),
            ProjectSection::Files => format!("/user/projects/{base}/files"),
            ProjectSection::Settings => format!("/user/projects/{base}/settings"),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ProjectSection::Dashboard => "Dashboard",
            ProjectSection::Team => "Team",
            ProjectSection::Files => "Files",
            ProjectSection::Settings => "Settings",
        }
    }
}

impl From<&str> for ProjectSection {
    fn from(path: &str) -> Self {
        match path {
            "team" => ProjectSection::Team,
            "files" => ProjectSection::Files,
            "settings" => ProjectSection::Settings,
            _ => ProjectSection::Dashboard,
        }
    }
}

impl Display for ProjectSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ProjectSection::Dashboard => "dashboard".to_string(),
            ProjectSection::Team => "team".to_string(),
            ProjectSection::Files => "files".to_string(),
            ProjectSection::Settings => "settings".to_string(),
        };
        write!(f, "{}", str)
    }
}


#[derive(Clone, Debug, PartialEq,Eq, Serialize,Deserialize)]
pub struct ProjectSlugSignal(pub String);




#[component]
pub fn ProjectPage() -> impl IntoView {
    let params: MemoProjectParams = use_params::<ProjectParams>();
    let project_slug =  move || {
        params.read()
            .as_ref()
            .ok()
            .and_then(|params|Some(params.project_slug.clone()))
            .unwrap_or_default()
    };
    let project_slug_signal = Signal::derive(move || {
        params
            .read()
            .as_ref()
            .map(|pp| ProjectSlugSignal(pp.project_slug.clone()))
            .expect("Project slug not found")
    });
    provide_context(project_slug_signal);
    
    let global_state:Store<GlobalState> = expect_context();
    let project_resource= OnceResource::new(get_project(project_slug()));
    let hosting_url_resource= OnceResource::new(get_hosting_url());
    
    
    let get_project_section = move |location:String| {
        let split = location.split("/").collect::<Vec<_>>();
        // find projets string and return next next
        let mut found_1 = false;
        let mut found_2 = false;
        for &s in split.iter() {
            if found_1 && !found_2 {
                found_2 = true;
            } else if found_2 {
                let p = ProjectSection::from(s);
                return p;
            } else if s.eq("projects") {
                found_1 = true;
            }
        }
        ProjectSection::default()
    };

    let location = use_location().pathname.get();
    // Gestion de la section active
    let (current, set_current): (ReadSignal<ProjectSection>, WriteSignal<ProjectSection>) =
        signal(get_project_section(location));
    Effect::new(move |_| {
        let location = use_location().pathname.get();
        let sec = get_project_section(location);
        set_current.set(sec);
    });

    view! {
        <div>
            <nav class="nav-main">
                <div class="nav-container">
                    <div class="nav-inner">
                        {move || {
                            ProjectSection::iter()
                                .map(|s| {
                                    view! {
                                        <SectionNav
                                            section=s
                                            current_section=current
                                            project_slug_signal
                                        />
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </div>
            </nav>
            <Suspense fallback=move || {
                view! {}
            }>
                {move || Suspend::new(async move {
                    let project = project_resource.await;
                    match project {
                        Ok(project) => {
                            global_state
                                .update(|inner| {
                                    inner.project = Some((project.get_slug(), project));
                                });
                        }
                        Err(_) => {
                            global_state.update(|inner| inner.project = None);
                        }
                    }
                    let hosting_url = hosting_url_resource.await;
                    match hosting_url {
                        Ok(hosting_url) => {
                            global_state.update(|inner| inner.hosting_url = Some(hosting_url));
                        }
                        Err(_) => {
                            global_state.update(|inner| inner.hosting_url = None);
                        }
                    }
                    view! { <Outlet /> }
                })}
            </Suspense>

        </div>
    }
}

#[component]
fn SectionNav(
    #[prop(into)] section: ProjectSection,
    #[prop(into)] current_section: ReadSignal<ProjectSection>,
    #[prop(into)] project_slug_signal: Signal<ProjectSlugSignal>,
) -> impl IntoView {
    view! {
        <A
            href=move || section.href(project_slug_signal.read().0.as_str())
            attr:class=move || {
                format!(
                    "nav-link {}",
                    if current_section() == section {
                        "nav-link-active"
                    } else {
                        "nav-link-inactive"
                    },
                )
            }
        >
            {section.label()}
        </A>
    }
}

pub mod server_fns {
    use crate::models::Project;
    use common::ProjectSlugStr;
    use leptos::prelude::ServerFnError;
    use leptos::server;



    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use common::permission::Permission;
    }}

    #[server]
    pub async fn get_project(
        project_slug: ProjectSlugStr,
    ) -> Result<Project, ServerFnError> {
        Ok(
            crate::security::permission::ssr::handle_project_permission_request(
                project_slug,
                Permission::Read,
                None,
                |_, pool, project_slug| async move {
                    let project = sqlx::query_as!(
                        Project,
                        "SELECT * FROM projects WHERE id = $1",
                        project_slug.id
                    )
                    .fetch_one(&pool)
                    .await?;
                    Ok(project)
                },
            )
            .await?,
        )
    }
}
