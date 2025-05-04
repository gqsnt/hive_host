use leptos::prelude::{
    expect_context, OnceResource, Read, Signal, Transition, Update,
};
use leptos::prelude::{AddAnyAttr, Suspend};
use std::fmt::Display;
pub mod project_dashboard;
pub mod project_files;
pub mod project_settings;
pub mod project_snapshots;
pub mod project_team;

use leptos::context::provide_context;
use leptos::prelude::{
    signal, ClassAttribute, CollectView, Effect, Get, Memo, ReadSignal, Set, WriteSignal,
};
use leptos::{component, view, IntoView, Params};
use leptos_router::hooks::{use_location, use_params};

use crate::app::get_hosting_url;
use crate::app::pages::user::projects::project::server_fns::get_project;
use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::IntoMaybeErased;
use leptos_router::components::{Outlet, A};
use leptos_router::params::{Params, ParamsError};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

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
    Snapshots,
    Settings,
}
impl ProjectSection {
    pub fn href(&self, base: &str) -> String {
        match self {
            ProjectSection::Dashboard => format!("/user/projects/{base}"),
            ProjectSection::Team => format!("/user/projects/{base}/team"),
            ProjectSection::Snapshots => format!("/user/projects/{base}/snapshots"),
            ProjectSection::Files => format!("/user/projects/{base}/files/root/"),
            ProjectSection::Settings => format!("/user/projects/{base}/settings"),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ProjectSection::Dashboard => "Dashboard",
            ProjectSection::Team => "Team",
            ProjectSection::Files => "Files",
            ProjectSection::Settings => "Settings",
            ProjectSection::Snapshots => "Snapshots",
        }
    }
}

impl From<&str> for ProjectSection {
    fn from(path: &str) -> Self {
        match path {
            "team" => ProjectSection::Team,
            "files" => ProjectSection::Files,
            "settings" => ProjectSection::Settings,
            "snapshots" => ProjectSection::Snapshots,
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
            ProjectSection::Snapshots => "snapshots".to_string(),
        };
        write!(f, "{str}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSlugSignal(pub String);

#[component]
pub fn ProjectPage(
) -> impl IntoView {
    let params: MemoProjectParams = use_params::<ProjectParams>();
    let project_slug = move || {
        params
            .read()
            .as_ref()
            .ok()
            .map(|p| p.project_slug.clone())
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

    let global_state: Store<GlobalState> = expect_context();
    let project_resource = OnceResource::new_bitcode(get_project(project_slug()));

    let hosting_url_resource = OnceResource::new_bitcode(get_hosting_url());

    let get_project_section = move |location: String| {
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
            <Transition>
                {move || Suspend::new(async move {
                    let project = project_resource.await;
                    match project {
                        Ok((permission, project)) => {
                            global_state
                                .project()
                                .update(|inner| {
                                    *inner = Some((project.get_slug(), permission, project));
                                });
                        }
                        Err(_) => {
                            global_state.project().update(|inner| *inner = None);
                        }
                    }
                    let hosting_url = hosting_url_resource.await;
                    match hosting_url {
                        Ok(hosting_url) => {
                            global_state.hosting_url().update(|inner| *inner = Some(hosting_url));
                        }
                        Err(_) => {
                            global_state.hosting_url().update(|inner| *inner = None);
                        }
                    }
                    view! { <Outlet /> }
                })}
            </Transition>

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
    use crate::AppResult;
    use common::website_to_server::permission::Permission;
    use common::ProjectSlugStr;
    use leptos::server;
    use leptos::server_fn::codec::Bitcode;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::security::utils::ssr::get_auth_session_user_id;
    }}

    #[server(input=Bitcode, output=Bitcode)]
    pub async fn get_project(project_slug: ProjectSlugStr) -> AppResult<(Permission, Project)> {
        crate::security::permission::ssr::handle_project_permission_request(
            project_slug,
            Permission::Read,
            None,
            |auth, pool, project_slug| async move {
                let user_id = get_auth_session_user_id(&auth).unwrap();
                let record = sqlx::query!(
                        r#"SELECT id,name,active_snapshot_id, slug, permissions.permission as "permission: Permission" FROM projects inner join permissions on projects.id = permissions.project_id and user_id = $1  WHERE id = $2"#,
                        user_id,
                        project_slug.id
                    )
                    .fetch_one(&pool)
                    .await?;
                let project = Project {
                    id: record.id,
                    name: record.name,
                    slug: record.slug,
                    active_snapshot_id: record.active_snapshot_id,
                };

                Ok((record.permission,project))
            },
        )
            .await
    }
}
