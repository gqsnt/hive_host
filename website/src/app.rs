use crate::api::fetch_api;
use crate::auth::{Login, Logout, Signup, User};
use crate::error_template::ErrorTemplate;
use crate::permission::{request_server_project_action, token_url};

use leptos::either::{Either, EitherOf4};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::server_fn::codec::IntoRes;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Outlet, ParentRoute, ProtectedParentRoute, A};
use leptos_router::hooks::use_params;
use leptos_router::params::Params;
use leptos_router::{components::{Route, Router, Routes}, path, MatchNestedRoutes, SsrMode};
use common::{ProjectId, ProjectSlug};
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use common::server_project_action::io_action::dir_action::DirAction;
use common::server_project_action::io_action::file_action::FileAction;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

pub type UserSignal = ReadSignal<Option<User>>;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    #[cfg(feature = "ssr")]
    crate::auth::ssr::set_headers();

    // Provides context that manages stylesheets, titles, meta tags, etc.
    let login = ServerAction::<Login>::new();
    let logout = ServerAction::<Logout>::new();
    let signup = ServerAction::<Signup>::new();

    let (user, set_user): (UserSignal, WriteSignal<Option<User>>) = signal(None::<User>);
    provide_context(user);

    Effect::new(move || match signup.value().get() {
        Some(Ok(user)) => set_user(Some(user)),
        _ => set_user(None),
    });
    Effect::new(move || match login.value().get() {
        Some(Ok(user)) => set_user(Some(user)),
        _ => set_user(None),
    });
    Effect::new(move || {
        let _ = logout.version().get();
        set_user(None);
    });

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/website.css" />

        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router>
            <nav>
                <A href="/">"Home"</A>
                {move || {
                    match user.read().is_some() {
                        true => {
                            Either::Left(
                                view! {
                                    <div>
                                        <A href="/user">"User"</A>
                                        <LogoutView action=logout />
                                    </div>
                                },
                            )
                        }
                        false => {
                            Either::Right(
                                view! {
                                    <div>
                                        <A href="/login">"Login"</A>
                                        <A  href="/signup">"Sign Up"</A>
                                    </div>
                                },
                            )
                        }
                    }
                }}
            </nav>
            <main>
                <Routes fallback=|| "Page not found.">
                    <Route path=path!("") view=HomePage />
                    <Route
                        path=path!("signup")
                        view=move || view! { <SignupView action=signup /> }
                    />

                    <Route path=path!("login") view=move || view! { <LoginView action=login /> } />
                    <UserRoutes/>
                </Routes>
            </main>
        </Router>
    }
}





#[component(transparent)]
fn UserRoutes() -> impl MatchNestedRoutes + Clone{
    let user = expect_context::<UserSignal>();
    view!{
        <ProtectedParentRoute
            path=path!("user")
            condition=move || Some(user.read().is_some())
            redirect_path=|| "/login"
            view=UserParentPage
        >
            <Route path=path!("") view=UserInfoPage />
            <ProjectRoutes/>

        </ProtectedParentRoute>
    }.into_inner()
}


#[component(transparent)]
fn ProjectRoutes() -> impl MatchNestedRoutes + Clone{
    view!{
        <ParentRoute
            path=path!("projects")
            view=ProjectsPage
        >
            <Route path=path!("") view=|| view! { "Select a project." } />
            <Route path=path!(":project_slug") view=ProjectPage ssr=SsrMode::Async />
        </ParentRoute>
    }.into_inner()
}



#[component]
fn UserParentPage() -> impl IntoView{
    let user = expect_context::<UserSignal>();
    view!{
        <h2>"User Parent"</h2>
        <div>
            <A href="/user/projects">"Projects"</A>
            <A href="/user">"User Info"</A>
        </div>
        <Outlet/>
    }
}



#[component]
fn UserInfoPage() -> impl IntoView{
    let user = expect_context::<UserSignal>();
    view!{
        <h3>"User Info"</h3>
    }
}



/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let count = RwSignal::new(0);
    let on_click = move |_| *count.write() += 1;

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}

#[component]
fn LoginView(action: ServerAction<Login>) -> impl IntoView {
    view! {
        <ActionForm action=action>
            <h1>"Log In"</h1>
            <CSRFField />
            <label>
                "Email:" <input type="email" placeholder="Email" maxlength="32" name="email" />
            </label>
            <br />
            <label>
                "Password:" <input type="password" placeholder="Password" name="password" />
            </label>
            <br />
            <label>
                <input type="checkbox" name="remember" />
                "Remember me?"
            </label>
            <br />
            <button type="submit">"Log In"</button>
        </ActionForm>
    }
}
#[component]
fn SignupView(action: ServerAction<Signup>) -> impl IntoView {
    view! {
        <ActionForm action=action>
            <h1>"Sign Up"</h1>
            <CSRFField />
            <label>
                "Email:" <input type="email" placeholder="Email" maxlength="32" name="email" />
            </label>
            <br />
            <label>
                "Username:" <input type="text" placeholder="Username" maxlength="32" name="username" />
            </label>
            <br />
            <label>
                "Password:" <input type="password" placeholder="Password" name="password" />
            </label>
            <br />
            <label>
                "Confirm Password:"
                <input type="password" placeholder="Password again" name="password_confirmation" />
            </label>
            <br />
            <label>
                "Remember me?" <input type="checkbox" name="remember" class="auth-input" />
            </label>

            <br />
            <button type="submit" class="button">
                "Sign Up"
            </button>
        </ActionForm>
    }
}

#[component]
pub fn LogoutView(action: ServerAction<Logout>) -> impl IntoView {
    view! {
        <div>
            <ActionForm action=action>
                <button type="submit" class="button">
                    "Log Out"
                </button>
            </ActionForm>
        </div>
    }
}

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
           return None;
        }
    })
}


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
                                        token_action.dispatch((get_project_slug(), DirAction::Tree.into()));
                                    }
                                    class="button"
                                >
                                    "Tree (no token)"
                                </button>
                                <button
                                    on:click=move |_| {
                                        token_action.dispatch((get_project_slug(), FileAction::View {path:"test.cat".to_string()}.into()));
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
                                            _ => {
                                                Either::Right(view! { <p>"No response"</p> })
                                            }
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

#[component]
pub fn CSRFField() -> impl IntoView {
    let csrf_resource = Resource::new(|| (), |_| generate_csrf());

    view! {
        <Transition fallback=|| {
            view! { <p>"Loading..."</p> }
        }>
            {move || {
                csrf_resource
                    .get()
                    .map(|n| match n {
                        Err(e) => {
                            Either::Left(
                                view! {
                                    {format!(
                                        "Page Load Failed: {e}. Please reload the page or try again later.",
                                    )}
                                },
                            )
                        }
                        Ok(csrf_hash) => {
                            Either::Right(
                                view! { <input type="hidden" name="csrf" value=csrf_hash /> },
                            )
                        }
                    })
            }}
        </Transition>
    }
}

#[server]
async fn generate_csrf() -> Result<String, ServerFnError> {
    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    let auth_session = auth.session.get_session_id().to_string();
    Ok(crate::auth::ssr::gen_easy_hash(
        auth_session,
        server_vars.csrf_server.to_secret(),
    ))
}
