use leptos::prelude::{CustomAttribute, Signal};
use leptos::prelude::{AddAnyAttr, ElementExt, For};
pub mod project_dashboard;
pub mod project_files;
pub mod project_settings;
pub mod project_team;

use std::fmt::format;
use leptos::{component, view, IntoView, Params};
use leptos::context::provide_context;
use leptos::either::{Either, EitherOf4};
use leptos::leptos_dom::log;
use leptos::prelude::{signal, Action, ClassAttribute, CollectView, Effect, Get, IntoAny, Memo, OnAttribute, ReadSignal, Resource, ServerFnError, Set, Suspend, Suspense, With, WriteSignal};
use leptos_router::hooks::{use_location, use_navigate, use_params};
use common::{ProjectSlug, ProjectSlugStr};
use common::server_project_action::io_action::dir_action::DirAction;
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use crate::security::permission::{request_server_project_action, token_url};
use leptos_router::params::{Params, ParamsError};
use leptos::prelude::ElementChild;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::IntoAnyAttribute;
use leptos_router::components::{Outlet, A};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use crate::app::pages::user::UserPage;

#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectParams {
    pub project_slug: String,
}



pub type MemoProjectParams = Memo<Result<ProjectParams,ParamsError>>;


#[derive(Default, Clone, Copy, PartialEq, Eq, EnumIter, Hash, Debug)]
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
        log!("Project Section not found");
        ProjectSection::default()
    };

    // Gestion de la section active
    let (current, set_current): (ReadSignal<ProjectSection>, WriteSignal<ProjectSection>) = signal(get_project_section());
    Effect::new(move |_| {
        let sec = get_project_section();
        set_current.set(sec);
    });

    // Classes communes
    let nav_bg = "bg-gray-800";
    let container = "mx-auto max-w-7xl px-2 sm:px-6 lg:px-8";
    let inner = "relative flex h-16 items-center justify-center space-x-4";

   
    
     view! {
         <nav class=nav_bg>
             <div class=container>
                 <div class=inner>
                     <SectionItem
                         section=ProjectSection::Dashboard
                         current_section=current
                         project_slug=get_project_slug
                     />
                     <SectionItem
                         section=ProjectSection::Files
                         current_section=current
                         project_slug=get_project_slug
                     />
                     <SectionItem
                         section=ProjectSection::Team
                         current_section=current
                         project_slug=get_project_slug
                     />
                     <SectionItem
                         section=ProjectSection::Settings
                         current_section=current
                         project_slug=get_project_slug
                     />
                 </div>
             </div>
         </nav>
         <Outlet />
     }
}





#[component]
fn SectionItem(
    #[prop(into)] section: ProjectSection,
    #[prop(into)] current_section: ReadSignal<ProjectSection>,
    #[prop(into)] project_slug: Signal<String>,
) -> impl IntoView {
    let base_classes= "rounded-md px-3 py-2 text-sm font-medium";
    let inactive_classes = "text-gray-300 hover:bg-gray-700 hover:text-white";
    let active_classes = "bg-gray-900 text-white";
    
    
    view! {
        <A
            href=section.href(project_slug.get().as_str())
            attr:class=move || {
                format!(
                    "{} {}",
                    base_classes,
                    if current_section() == section { active_classes } else { inactive_classes },
                )
            }
        >
            {section.label()}
        </A>
    }
}

fn get_action_server_project_action(
) -> Action<(ProjectSlugStr, ServerProjectAction), Option<ServerProjectActionResponse>>{
    Action::new(|input: &(ProjectSlugStr, ServerProjectAction)| {
        let (project_slug, action) = input.clone();
        async move {
            if let Ok(r) = request_server_project_action(project_slug, action).await{
                return if let ServerProjectActionResponse::Token(token) = r.clone(){
                    crate::api::fetch_api(token_url(token).as_str()).await
                }else{
                    Some(r)
                }
            }
             None
        }
    })
}


