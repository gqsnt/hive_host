pub mod pages;
pub mod components;



use leptos::either::{Either};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{ParentRoute, ProtectedParentRoute, A};
use leptos_router::{components::{Route, Router, Routes}, path, MatchNestedRoutes, SsrMode};
use crate::app::components::logout_button::LogoutButton;
use crate::app::pages::home::HomePage;
use crate::app::pages::login::LoginPage;
use crate::app::pages::signup::SignupPage;
use crate::app::pages::user::projects::project::ProjectPage;
use crate::app::pages::user::projects::ProjectsPage;
use crate::app::pages::user::user_info::UserInfoPage;
use crate::app::pages::user::UserPage;
use crate::security::login::Login;
use crate::security::{get_user, Logout, User};
use crate::security::signup::Signup;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html class="h-full bg-white" lang="en">
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

pub type ReadUserSignal = ReadSignal<Option<User>>;
pub type WriteUserSignal = WriteSignal<Option<User>>;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    #[cfg(feature = "ssr")]
    crate::security::utils::ssr::set_headers();

    // Provides context that manages stylesheets, titles, meta tags, etc.
    let login = ServerAction::<Login>::new();
    let logout = ServerAction::<Logout>::new();
    let signup = ServerAction::<Signup>::new();
    let user_data = Resource::new(
        move || {},
        move |_| get_user(),
    );


    let (user, set_user): (ReadUserSignal, WriteUserSignal) = signal(None::<User>);
    provide_context(user);
    provide_context(set_user);




    Effect::new(move || match user_data.get() {
        Some(Ok(user)) => set_user(user.clone()),
        _ => set_user(None),
    });

    Effect::new(move || match login.value().get() {
        Some(Ok(user)) => set_user(Some(user)),
        _ => set_user(None),
    });
    Effect::new(move || match signup.value().get() {
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
            <div>
                <header class="absolute inset-x-0 top-0 z-50">
                    <nav class="flex items-center justify-between p-6 lg:px-8">
                        <A attr:class="-m-1.5 p-1.5" href="/">
                            <span class="sr-only">Hive Host</span>
                            <img
                                class="h-8 w-auto"
                                src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                                alt=""
                            />
                        </A>
                        {move || {
                            match user.read().is_some() {
                                true => {
                                    Either::Left(
                                        view! {
                                            <div>

                                                <div class="hidden lg:flex lg:flex-1 lg:justify-end">
                                                    <A
                                                        href="/user"
                                                        attr:class="text-sm/6 font-semibold text-gray-900"
                                                    >
                                                        User
                                                    </A>
                                                </div>

                                                <LogoutButton action=logout />
                                            </div>
                                        },
                                    )
                                }
                                false => {
                                    Either::Right(
                                        view! {
                                            <div>
                                                <div class="hidden lg:flex lg:flex-1 lg:justify-end">
                                                    <A
                                                        href="/login"
                                                        attr:class="text-sm/6 font-semibold text-gray-900"
                                                    >
                                                        Log in ->
                                                    </A>
                                                </div>

                                            </div>
                                        },
                                    )
                                }
                            }
                        }}
                    </nav>
                </header>

                <main class="isolate">
                    <div class="relative pt-14">
                        <Routes fallback=|| "Page not found.">
                            <Route path=path!("") view=HomePage />
                            <Route
                                path=path!("signup")
                                view=move || view! { <SignupPage action=signup /> }
                            />

                            <Route
                                path=path!("login")
                                view=move || view! { <LoginPage action=login /> }
                            />
                            <UserRoutes user_data />
                        </Routes>
                    </div>
                </main>
            </div>
        </Router>
    }
}





#[component(transparent)]
fn UserRoutes(user_data:Resource<Result<Option<User>, ServerFnError>>) -> impl MatchNestedRoutes + Clone{
    let set_user = expect_context::<WriteUserSignal>();
    let user_signal = expect_context::<ReadUserSignal>();
    view! {
        <ProtectedParentRoute
            path=path!("user")
            condition=move || {
                Some(
                    match user_signal.get().is_some() {
                        true => true,
                        false => {
                            match user_data.get() {
                                Some(Ok(Some(user))) => {
                                    set_user(Some(user));
                                    true
                                }
                                _ => false,
                            }
                        }
                    },
                )
            }
            redirect_path=|| "/login"
            view=UserPage
        >
            <Route path=path!("") view=UserInfoPage />
            <ProjectRoutes />

        </ProtectedParentRoute>
    }.into_inner()
}



#[component(transparent)]
fn ProjectRoutes() -> impl MatchNestedRoutes + Clone{
    view! {
        <ParentRoute path=path!("projects") view=ProjectsPage>
            <Route path=path!("") view=|| view! { "Select a project." } />
            <Route path=path!(":project_slug") view=ProjectPage ssr=SsrMode::Async />
        </ParentRoute>
    }.into_inner()
}

