use leptos::{component, view, IntoView};
use leptos::either::Either;
use leptos::prelude::{CollectView, ErrorBoundary, Resource, Suspend, Suspense};
use leptos_router::components::{Outlet, A};
use crate::error_template::ErrorTemplate;
use leptos::prelude::ElementChild;

pub mod project;



#[component]
pub fn ProjectsPage() -> impl IntoView {
    let projects = Resource::new_blocking(move || (), move |_| crate::projects::get_projects());
    view! {
        <div>
            <h2>"Projects"</h2>
            <Suspense fallback=|| "Loading...".into_view()>
                <ErrorBoundary fallback=|errors| {
                    view! { <ErrorTemplate errors=errors /> }
                }>

                    {move || Suspend::new(async move {
                        match projects.await {
                            Ok(projects) => {
                                Either::Right({
                                    if projects.is_empty() {
                                        Either::Left(

                                            view! { <p>"No projects found."</p> },
                                        )
                                    } else {
                                        Either::Right(
                                            projects
                                                .into_iter()
                                                .map(move |project| {
                                                    view! {
                                                        <div>
                                                            <h2>{format!("Project {}", project.name)}</h2>
                                                            <A href=format!(
                                                                "/user/projects/{}",
                                                                project.get_slug().to_str(),
                                                            )>"View Project"</A>
                                                        </div>
                                                    }
                                                })
                                                .collect_view(),
                                        )
                                    }
                                })
                            }
                            Err(_) => Either::Left(()),
                        }
                    })} <Outlet />
                </ErrorBoundary>

            </Suspense>

        </div>
    }
}


