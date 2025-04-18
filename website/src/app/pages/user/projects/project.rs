use std::fmt::format;
use leptos::{component, view, IntoView, Params};
use leptos::either::{Either, EitherOf4};
use leptos::leptos_dom::log;
use leptos::prelude::{Action, ClassAttribute, Get, IntoAny, OnAttribute, Resource, ServerFnError, Suspend, Suspense};
use leptos_router::hooks::{use_navigate, use_params};
use common::{ProjectSlug, ProjectSlugStr};
use common::server_project_action::io_action::dir_action::DirAction;
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use crate::security::permission::{request_server_project_action, token_url};
use leptos_router::params::Params;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::IntoAnyAttribute;

#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectParams {
    pub project_slug: String,
}


#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params::<ProjectParams>();
    let get_project_slug = move || {
        params
            .get()
            .map(|pp| pp.project_slug)
            .expect("Project slug not found")
    };
    let project = Resource::new(get_project_slug, move |project_slug| {
        crate::projects::get_project(project_slug)
    });

    view! {
        <Suspense fallback=|| {
            "Loading...".into_view()
        }>
            {move || Suspend::new(async move {
                match project.await {
                    Ok(project) => {
                        let project_slug = project.get_slug();
                        Either::Left({
                            let token_action = get_action_server_project_action();
                            let token_responce = token_action.value();

                            view! {
                                <h3>Project {project.name}</h3>
                                <button
                                    on:click=move |_| {
                                        token_action
                                            .dispatch((get_project_slug(), DirAction::Tree.into()));
                                    }
                                    class="button"
                                >
                                    "Tree (no token)"
                                </button>
                                <button
                                    on:click=move |_| {
                                        token_action
                                            .dispatch((
                                                get_project_slug(),
                                                FileAction::View {
                                                    path: "test.cat".to_string(),
                                                }
                                                    .into(),
                                            ));
                                    }
                                    class="button"
                                >
                                    "Get File (token)"
                                </button>

                                <p>
                                    {move || {
                                        match token_responce.get() {
                                            Some(Some(response)) => {
                                                match response {
                                                    ServerProjectActionResponse::Ok => "Ok".to_string(),
                                                    ServerProjectActionResponse::Token(s) => {
                                                        format!("Token: {}", s)
                                                    }
                                                    ServerProjectActionResponse::Content(content) => {
                                                        format!("Content: {}", content)
                                                    }
                                                    ServerProjectActionResponse::Tree(tree) => {
                                                        format!("Tree: {:?}", tree)
                                                    }
                                                }
                                            }
                                            _ => "No response".to_string(),
                                        }
                                    }}
                                </p>
                            }
                        })
                    }
                    Err(e) => Either::Right(e.to_string().into_view()),
                }
            })}
        </Suspense>
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


