pub mod components;
pub mod pages;

use crate::app::pages::home::HomePage;
use crate::app::pages::login::LoginPage;
use crate::app::pages::signup::SignupPage;
use crate::app::pages::user::dashboard::DashboardPage;
use crate::app::pages::user::projects::new_project::{server_fns::CreateProject, NewProjectPage};
use crate::app::pages::user::projects::project::project_dashboard::ProjectDashboard;
use crate::app::pages::user::projects::project::project_files::ProjectFiles;
use crate::app::pages::user::projects::project::project_settings::ProjectSettings;
use crate::app::pages::user::projects::project::project_snapshots::ProjectSnapshots;
use crate::app::pages::user::projects::project::project_team::ProjectTeam;
use crate::app::pages::user::projects::project::ProjectPage;
use crate::app::pages::user::projects::ProjectsPage;
use crate::app::pages::user::user_settings::UserSettingsPage;
use crate::app::pages::user::UserPage;
use crate::app::pages::GlobalState;
use crate::AppResult;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::ParentRoute;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html class="h-full bg-gray-900" lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body class="h-full">
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    #[cfg(feature = "ssr")]
    crate::security::utils::ssr::set_headers();

    provide_context(Store::new(GlobalState::default()));

    // Provides context that manages stylesheets, titles, meta tags, etc.

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/website.css" />

        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router>
            <div class="h-full">
                <main class="h-full">
                    <div class="h-full">
                        <Routes fallback=|| "Page not found.">
                            <Route path=path!("") view=HomePage />
                            <Route path=path!("signup") view=SignupPage />

                            <Route path=path!("login") view=LoginPage />
                            <UserRoutes />
                        </Routes>
                    </div>
                </main>
            </div>
        </Router>
    }
}

#[component(transparent)]
fn UserRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("user") view=UserPage>
            <Route path=path!("") view=DashboardPage />
            <Route path=path!("settings") view=UserSettingsPage />
            <ProjectsRoutes />

        </ParentRoute>
    }
    .into_inner()
}

#[component(transparent)]
fn ProjectsRoutes() -> impl MatchNestedRoutes + Clone {
    let create_project_action = ServerAction::<CreateProject>::new();
    view! {
        <ParentRoute
            path=path!("projects")
            view=move || view! { <ProjectsPage create_project_action /> }
        >
            <Route path=path!("") view=move || view! { <NewProjectPage create_project_action /> } />
            <ProjectRoutes />
        </ParentRoute>
    }
    .into_inner()
}

#[component(transparent)]
fn ProjectRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute
            path=path!(":project_slug")
            view=ProjectPage>
            <Route path=path!("") view=ProjectDashboard />
            <Route path=path!("settings") view=ProjectSettings />
            <Route path=path!("files/*path") view=ProjectFiles />
            <Route path=path!("team") view=ProjectTeam />
            <Route
                path=path!("snapshots")
                view=ProjectSnapshots
            />
        </ParentRoute>
    }
    .into_inner()
}

#[server]
pub async fn get_server_url() -> AppResult<String> {
    use crate::ssr::server_vars;
    let server_vars = server_vars()?;
    Ok(server_vars.server_url.to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostingUrl(pub String);

#[server]
pub async fn get_hosting_url() -> AppResult<String> {
    use crate::ssr::server_vars;
    let server_vars = server_vars()?;
    Ok(server_vars.hosting_url.to_string())
}
