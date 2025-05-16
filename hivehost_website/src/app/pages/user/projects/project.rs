use leptos::prelude::{expect_context, Read, Resource, Signal, Transition, Update};
use leptos::prelude::{AddAnyAttr, Suspend};
use std::fmt::Display;
pub mod project_dashboard;
pub mod project_files;
pub mod project_settings;
pub mod project_snapshots;
pub mod project_team;

use leptos::context::provide_context;
use leptos::prelude::{
     ClassAttribute, CollectView, Get, Memo,
};
use leptos::{component, view, IntoView, Params};
use leptos_router::hooks::{use_location, use_params};

use crate::app::pages::user::projects::project::server_fns::get_project;
use crate::app::pages::{GlobalState, GlobalStateStoreFields, ProjectState};
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

    pub fn from_first_segment(segment:&str)->Self{
        match segment {
            "team" => ProjectSection::Team,
            "files" => ProjectSection::Files,
            "settings" => ProjectSection::Settings,
             _  => ProjectSection::Snapshots,
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
    let global_state: Store<GlobalState> = expect_context();
    let project_slug_signal = Signal::derive(move || {
        params
            .read()
            .as_ref()
            .map(|pp| ProjectSlugSignal(pp.project_slug.clone()))
            .expect("Project slug not found")
    });
    provide_context(project_slug_signal);

    #[allow(clippy::redundant_closure)]
    let project_resource = Resource::new_bincode(

        move || project_slug_signal() ,
        move |s  | get_project(s.0),
    );
    
    let active_project_section = Memo::new(move |_| {
        let current_path = use_location().pathname.get();
        let segments: Vec<&str> = current_path.split('/').filter(|s| !s.is_empty()).collect();

        if let Some(projects_idx) = segments.iter().position(|&seg| seg == "projects") {
            if segments.len() > projects_idx + 2 {
                ProjectSection::from_first_segment(segments[projects_idx + 2])
            } else {
                ProjectSection::default()
            }
        } else {
            leptos::logging::warn!(
                "Could not determine project section: 'projects' segment not found in path: {}",
                current_path
            );
            ProjectSection::default()
        }
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
                                            current_section=active_project_section
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
                                .project_state()
                                .update(|inner| {
                                    *inner = Some(
                                        ProjectState::new(project.get_slug(), permission, project),
                                    );
                                });
                        }
                        Err(_) => {
                            global_state.project_state().update(|inner| *inner = None);
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
    #[prop(into)] current_section: Memo<ProjectSection>,
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
    use crate::models::{Project};
    use crate::AppResult;
    use common::server_action::permission::Permission;
    use common::ProjectSlugStr;
    use leptos::server;
    use leptos::server_fn::codec::Bincode;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::models::GitProject;
        use crate::security::utils::ssr::get_auth_session_user_id;
    }}

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_project(project_slug: ProjectSlugStr) -> AppResult<(Permission, Project)> {
        crate::security::permission::ssr::handle_project_permission_request(
            project_slug,
            Permission::Read,
            None,
            |auth, pool, project_slug| async move {
                let user_id = get_auth_session_user_id(&auth).unwrap();
                let record = sqlx::query!(
                        r#"SELECT 
                            pr.id,
                            pr.name,
                            pr.active_snapshot_id, 
                            pr.slug, 
                            pe.permission as "permission: Permission", 
                            pr.server_id as server_id, 
                            s.hosting_address as hosting_address,
                            pr.project_github_id as project_github_id,
                            pgi.repo_full_name as "repo_full_name?: String",
                            pgi.branch_name as "branch_name?: String",
                            pgi.dev_commit as "dev_commit?: String",
                            pgi.last_commit as "last_commit?: String",
                            pgi.auto_deploy as "auto_deploy?: bool",
                            ug.installation_id as "installation_id?: i64",
                            ug.id as "user_githubs_id?: i64"
                        FROM projects pr 
                            inner join servers s on pr.server_id = s.id  
                            inner join permissions pe on pr.id = pe.project_id and pe.user_id = $1 
                            left join projects_github  pgi on pr.project_github_id = pgi.id
                            left join user_githubs ug on ug.id = pgi.user_githubs_id
                        WHERE pr.id = $2"#,
                        user_id,
                        project_slug.id
                    )
                    .fetch_one(&pool)
                    .await?;
                let prod_branch_commit = if let Some(snapshot_id) = record.active_snapshot_id {
                    let ps_record = sqlx::query!(
                        r#"SELECT git_commit, git_branch
                        FROM projects_snapshots
                        WHERE id = $1"#,
                        snapshot_id
                    )
                    .fetch_one(&pool)
                    .await?;
                    if let (Some(git_commit), Some(git_branch)) = (ps_record.git_commit, ps_record.git_branch) {
                        Some((git_branch, git_commit))
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                    
                let git_project = if record.project_github_id.is_some(){
                    Some(GitProject{
                        id: record.project_github_id.unwrap(),
                        repo_full_name: record.repo_full_name.unwrap(),
                        branch_name: record.branch_name.unwrap(),
                        dev_commit: record.dev_commit.unwrap(),
                        prod_branch_commit,
                        last_commit: record.last_commit.unwrap(),
                        auto_deploy:record.auto_deploy.unwrap(),
                        installation_id: record.installation_id.unwrap(),
                        user_githubs_id: record.user_githubs_id.unwrap(),
                    })
                }else{
                    None
                };
                let project = Project {
                    id: record.id,
                    name: record.name,
                    slug: record.slug,
                    active_snapshot_id: record.active_snapshot_id,
                    server_id: record.server_id,
                    hosting_address: record.hosting_address,
                    git_project
                };

                Ok((record.permission,project))
            },
        )
            .await
    }
}
