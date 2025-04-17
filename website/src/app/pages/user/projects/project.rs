use leptos::{component, view, IntoView, Params};
use leptos::either::{Either, EitherOf4};
use leptos::leptos_dom::log;
use leptos::prelude::{Action, ClassAttribute, Get, OnAttribute, Resource, Suspend, Suspense};
use leptos_router::hooks::use_params;
use common::ProjectSlug;
use common::server_project_action::io_action::dir_action::DirAction;
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use crate::api::fetch_api;
use crate::security::permission::{request_server_project_action, token_url};
use leptos_router::params::Params;
use leptos::prelude::ElementChild;


#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectParams {
    pub project_slug: ProjectSlug,
}


#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params::<ProjectParams>();
    let get_project_slug = move || {
        params
            .get()
            .map(|pp| pp.project_slug)
            .unwrap()
    };
    let project = Resource::new_blocking(get_project_slug, move |project_slug| {
        log!("running here {}", project_slug.id);
        crate::projects::get_project(project_slug.id)
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
                                                Either::Left(
                                                    match response {
                                                        ServerProjectActionResponse::Ok => {
                                                            EitherOf4::A(

                                                                view! { <p>"Ok"</p> },
                                                            )
                                                        }
                                                        ServerProjectActionResponse::Token(s) => {
                                                            EitherOf4::B(view! { <p>Token: {s}</p> })
                                                        }
                                                        ServerProjectActionResponse::Content(content) => {
                                                            EitherOf4::C(view! { <p>Content: {content}</p> })
                                                        }
                                                        ServerProjectActionResponse::Tree(tree) => {
                                                            EitherOf4::D(view! { <p>Tree:</p> })
                                                        }
                                                    },
                                                )
                                            }
                                            _ => Either::Right(view! { <p>"No response"</p> }),
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
) -> Action<(ProjectSlug, ServerProjectAction), Option<ServerProjectActionResponse>>{
    Action::new(|input: &(ProjectSlug, ServerProjectAction)| {
        let (project_slug, action) = input.clone();
        async move {
            if let Ok(r) = request_server_project_action(project_slug, action).await{
                return if let ServerProjectActionResponse::Token(token) = r.clone(){
                    fetch_api(token_url(token).as_str()).await
                }else{
                    Some(r)
                }
            }
             None
        }
    })
}


