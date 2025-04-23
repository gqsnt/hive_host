use leptos::prelude::{signal, ClassAttribute, OnAttribute};
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::Get;
use crate::app::IntoView;
use leptos::prelude::{expect_context, Action, ElementChild, ServerFnError, Signal};
use leptos::{component, server, view};
use common::hosting_action::HostingAction;
use common::permission::Permission;
use common::ProjectSlugStr;

use crate::app::pages::CsrfValue;
use crate::app::pages::user::projects::project::MemoProjectParams;
use crate::models::Project;

#[component]
pub fn ProjectSettings() -> impl IntoView {

    let params: MemoProjectParams = expect_context();
    //let project:Project = expect_context();

    let (is_active, set_is_active) = signal(false);

    let slug = Signal::derive(move || params.get().unwrap().project_slug.clone());
    let toggle_project = Action::new(
        |intput :&(ProjectSlugStr, String, bool)| {
            let (project_slug, csrf, is_active) = intput.clone();
            async move {
                toggle_project_active(csrf, project_slug, is_active).await
            }
        },
    );
    
    let clear_cache_action = Action::new(
        |intput :&(ProjectSlugStr, String)| {
            let (project_slug, csrf) = intput.clone();
            async move {
                on_clear_cache(csrf, project_slug).await
            }
        },
    );
    let reload_project_action = Action::new(
        |intput :&(ProjectSlugStr, String)| {
            let (project_slug, csrf) = intput.clone();
            async move {
                on_reload_project(csrf, project_slug).await
            }
        },
    );
    
    let csrf_value = expect_context::<Signal<CsrfValue>>();

    let on_toggle_project = move |_| {
        let project_slug = slug.get();
        let csrf = csrf_value.get().0.clone();
        set_is_active(!is_active.get());
        toggle_project.dispatch((project_slug,csrf, is_active.get()));
    };
    let on_reload_project = move |_| {
        let project_slug = slug.get();
        let csrf = csrf_value.get().0.clone();
        reload_project_action.dispatch((project_slug, csrf));
    };
    
    let on_clear_cache = move |_| {
        let project_slug = slug.get();
        let csrf = csrf_value.get().0.clone();
        clear_cache_action.dispatch((project_slug, csrf));
    };

    view! {
        <div>
            <h2>Project Settings</h2>
            <p>Manage your project settings here.</p>
            <p>Projet is {if is_active.get() {"active"} else {"inactive"}}</p>
            <button class="mt-2 btn-primary" on:click=on_toggle_project>Toggle Project</button>
            <button class="ml-2 btn-primary" on:click=on_reload_project>Reload Project</button>
            <button class="ml-2 btn-primary" on:click=on_clear_cache>Clear Cache</button>
        </div>
    }
}


#[server]
pub async fn toggle_project_active(csrf:String, project_slug: ProjectSlugStr, is_active:bool) -> Result<(), ServerFnError> {
    use crate::security::permission::ssr::handle_project_permission_request;
    use crate::api::ssr::request_hosting_action;
    handle_project_permission_request(
        project_slug,
       Permission::Owner,
        Some(csrf),
        |_,db ,project_slug|async move{

            let project = sqlx::query!(
                "UPDATE projects SET is_active = $1 WHERE id = $2",
                is_active,
                project_slug.id
            ).execute(&db).await?;
            let action = if is_active{
                HostingAction::ServeReloadProject
            }else{
                HostingAction::StopServingProject
            };
            request_hosting_action(
                project_slug,
                action
            ).await?;
            Ok(())
        }
    ).await
}


#[server]
pub async fn on_reload_project(csrf:String, project_slug: ProjectSlugStr) -> Result<(), ServerFnError> {
    use crate::security::permission::ssr::handle_project_permission_request;
    use crate::api::ssr::request_hosting_action;
    handle_project_permission_request(
        project_slug,
       Permission::Owner,
        Some(csrf),
        |_,_ ,project_slug|async move{
            let action = HostingAction::ServeReloadProject;
            request_hosting_action(
                project_slug,
                action
            ).await?;
            Ok(())
        }
    ).await
}

#[server]
pub async fn on_clear_cache(csrf:String, project_slug: ProjectSlugStr) -> Result<(), ServerFnError> {
    use crate::security::permission::ssr::handle_project_permission_request;
    use crate::api::ssr::request_hosting_action;
    handle_project_permission_request(
        project_slug,
       Permission::Owner,
        Some(csrf),
        |_,_ ,project_slug|async move{
            let action = HostingAction::ClearCache;
            request_hosting_action(
                project_slug,
                action
            ).await?;
            Ok(())
        }
    ).await
}
