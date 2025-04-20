use leptos::prelude::{CustomAttribute, Signal};
use leptos::prelude::{AddAnyAttr, ElementExt, For};
pub mod project_dashboard;
pub mod project_files;
pub mod project_settings;
pub mod project_team;

use std::fmt::format;
use leptos::{component, server, view, IntoView, Params};
use leptos::context::provide_context;
use leptos::either::{Either, EitherOf4};
use leptos::leptos_dom::log;
use leptos::prelude::{signal, Action, ClassAttribute, CollectView, Effect, Get, IntoAny, Memo, OnAttribute, ReadSignal, Resource, ServerFnError, Set, Suspend, Suspense, With, WriteSignal};
use leptos_router::hooks::{use_location, use_navigate, use_params};
use common::{ProjectSlug, ProjectSlugStr};
use common::server_project_action::io_action::dir_action::DirAction;
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use crate::security::permission::{token_url};
use leptos_router::params::{Params, ParamsError};
use leptos::prelude::ElementChild;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::IntoAnyAttribute;
use leptos_router::components::{Outlet, A};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use crate::app::pages::user::UserPage;
use crate::models::Project;

#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectParams {
    pub project_slug: String,
}



pub type MemoProjectParams = Memo<Result<ProjectParams,ParamsError>>;


#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug, EnumIter)]
pub enum ProjectSection {
    #[default]
    Dashboard,
    Team,
    Files,
    Settings,
}
impl ProjectSection {
    pub fn href(&self,base:&str) -> String {
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
        match path{
            "team" => ProjectSection::Team,
            "files" => ProjectSection::Files,
            "settings" => ProjectSection::Settings,
            _ => ProjectSection::Dashboard,
        }
    }
}

impl ToString for ProjectSection {
    fn to_string(&self) -> String {
        match self {
            ProjectSection::Dashboard => "dashboard".to_string(),
            ProjectSection::Team => "team".to_string(),
            ProjectSection::Files => "files".to_string(),
            ProjectSection::Settings => "settings".to_string(),
        }
    }
}



#[component]
pub fn ProjectPage() -> impl IntoView {
    let params:MemoProjectParams = use_params::<ProjectParams>();
    provide_context(params);
    let get_project_slug = Signal::derive(move || {
        params
            .get()
            .map(|pp| pp.project_slug)
            .expect("Project slug not found")
    });


    let get_project_section =move ||{
        let location = use_location().pathname.get().clone();
        let split = location.split("/").collect::<Vec<_>>();
        // find projets string and return next next
        let mut found_1 = false;
        let mut found_2 = false;
        for &s in split.iter() {
            if found_1 && !found_2{
                found_2 = true;
            }
            else if found_2 {
                let p = ProjectSection::from(s);
                return p;
            }
            else if s.eq("projects") {
                found_1 = true;
            }

        }
        ProjectSection::default()
    };

    // Gestion de la section active
    let (current, set_current): (ReadSignal<ProjectSection>, WriteSignal<ProjectSection>) = signal(get_project_section());
    Effect::new(move |_| {
        let sec = get_project_section();
        set_current.set(sec);
    });
    
     view! {
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
                                         project_slug=get_project_slug
                                     />
                                 }
                             })
                             .collect_view()
                     }}
                 </div>
             </div>
         </nav>
         <Outlet />
     }
}





#[component]
fn SectionNav(
    #[prop(into)] section: ProjectSection,
    #[prop(into)] current_section: ReadSignal<ProjectSection>,
    #[prop(into)] project_slug: Signal<String>,
) -> impl IntoView {

    
    view! {
        <A
            href=move || section.href(project_slug.get().as_str())
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

#[server]
pub async fn get_project(project_slug:ProjectSlugStr) -> Result<Project, ServerFnError> {
    
    use common::permission::Permission;
    use std::str::FromStr;
    use crate::security::permission::PermissionResult;
    
    let (_, pool, project_slug ) = crate::security::permission::ssr::handle_project_permission_request(project_slug, Permission::Read).await?;
    let project = sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", project_slug.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(project)
}