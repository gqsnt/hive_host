use crate::app::pages::include_csrf;
use crate::app::ReadUserSignal;
use crate::models::User;
use crate::security::{get_user, Logout};
use leptos::context::provide_context;
use leptos::either::Either;
use leptos::form::ActionForm;
use leptos::prelude::CustomAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::{signal, ClassAttribute, Get, GlobalAttributes};
use leptos::prelude::{AddAnyAttr, Effect, ReadSignal, ServerAction, Suspense, WriteSignal};
use leptos::prelude::{AriaAttributes, OnAttribute};
use leptos::server::OnceResource;
use leptos::{component, view, IntoView};
use leptos_router::components::{Outlet, A};
use leptos_router::hooks::use_location;

pub mod dashboard;
pub mod projects;
pub mod user_settings;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum UserPage {
    #[default]
    Dashboard,
    Projects,
    Settings,
}

impl UserPage {
    pub fn href(&self) -> String {
        match self {
            UserPage::Dashboard => "/user".to_string(),
            UserPage::Projects => "/user/projects".to_string(),
            UserPage::Settings => "/user/settings".to_string(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            UserPage::Dashboard => "Dashboard".to_string(),
            UserPage::Projects => "Projects".to_string(),
            UserPage::Settings => "Settings".to_string(),
        }
    }

    pub fn svg(&self) -> String {
        match self {
            UserPage::Dashboard => "m2.25 12 8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25".to_string(),
            UserPage::Projects => "M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z".to_string(),
            UserPage::Settings =>     "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z".to_string(),
        }
    }
}

impl From<&str> for UserPage {
    fn from(page: &str) -> Self {
        let paths = page.trim_start_matches('/').split('/').collect::<Vec<_>>();
        match paths.get(1) {
            None => UserPage::default(),
            Some(&path) => match path {
                "dashboard" => UserPage::Dashboard,
                "projects" => UserPage::Projects,
                "settings" => UserPage::Settings,
                _ => UserPage::default(),
            },
        }
    }
}

#[component]
pub fn UserPage() -> impl IntoView {
    let logout = ServerAction::<Logout>::new();
    let user_resource = OnceResource::new(get_user());
    include_csrf();

    let (mobile_sidebar_open, set_mobile_sidebar_open) = signal(false);
    let (user_signal, set_user_signal): (ReadUserSignal, WriteSignal<User>) =
        signal(User::default());
    provide_context(user_signal);
    let location = use_location();
    let (current_page, set_current_page) = signal(UserPage::from(location.pathname.get().as_str()));

    Effect::new(move |_| {
        let path = location.pathname.get();
        set_current_page(UserPage::from(path.as_str()));
    });

    view! {
        <Suspense fallback=move || {
            view! { <p>Loading ...</p> }
        }>
            {move || {
                user_resource
                    .map(|data| {
                        match data {
                            Ok(Some(user)) => {
                                set_user_signal(user.clone());
                                let menu_icon_path = "M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5"
                                    .to_string();
                                let close_icon_path = "M6 18 18 6M6 6l12 12".to_string();
                                let logout_icon_path = "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9"
                                    .to_string();
                                let logout_icon_path2 = logout_icon_path.clone();
                                Either::Right(
                                    view! {
                                        <div class="dark h-full"> // Add h-full if body/html are h-full
                                            <div
                                                class="relative z-50 lg:hidden"
                                                role="dialog"
                                                aria-modal="true"
                                                class:hidden=move || !mobile_sidebar_open.get()
                                            >
                                                // Backdrop
                                                <div
                                                    class="fixed inset-0 bg-gray-950/80 transition-opacity ease-linear duration-300"
                                                    class:opacity-100=move || mobile_sidebar_open.get()
                                                    class:opacity-0=move || !mobile_sidebar_open.get()
                                                    aria-hidden="true"
                                                    on:click=move |_| set_mobile_sidebar_open(false)
                                                ></div>

                                                <div class="fixed inset-0 flex">

                                                    <div
                                                        class="relative mr-16 flex w-full max-w-xs flex-1 transform transition ease-in-out duration-300"
                                                        class:translate-x-0=move || mobile_sidebar_open.get()
                                                        class:-translate-x-full=move || !mobile_sidebar_open.get()
                                                    >

                                                        <div
                                                            class="absolute top-0 left-full flex w-16 justify-center pt-5 transform transition ease-in-out duration-300"
                                                           class:opacity-100=move || mobile_sidebar_open.get()
                                                           class:opacity-0=move || !mobile_sidebar_open.get()
                                                        >
                                                            <button
                                                                type="button"
                                                                class="-m-2.5 p-2.5"
                                                                on:click=move |_| set_mobile_sidebar_open(false)
                                                            >
                                                                <span class="sr-only">"Close sidebar"</span>
                                                                <svg class="size-6 text-white" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" d=close_icon_path></path>
                                                                </svg>
                                                            </button>
                                                        </div>


                                                        <div class="flex grow flex-col gap-y-5 overflow-y-auto bg-gray-950 px-6 pb-4 ring-1 ring-white/10">
                                                            <div class="flex h-16 shrink-0 items-center">
                                                                 <A href="/">
                                                         <img class="h-8 w-auto" src="/favicon.ico" alt="Your Company"/> // Update logo path
                                                            </A>
                                                            </div>
                                                            <nav class="flex flex-1 flex-col">
                                                                <ul role="list" class="flex flex-1 flex-col gap-y-7">
                                                                    <li>
                                                                        <ul role="list" class="-mx-2 space-y-1">
                                                                            <NavItem page=UserPage::Dashboard current_page=current_page/> // Pass ReadSignal
                                                                            <NavItem page=UserPage::Projects current_page=current_page/>
                                                                        </ul>
                                                                    </li>


                                                                    <div class="mt-auto -mx-2 space-y-1">
                                                                        <li>
                                                                            <A
                                                                                href=UserPage::Settings.href()
                                                                                attr:class="group flex gap-x-3 rounded-md p-2 text-sm/6 font-semibold"
                                                                                class:bg-gray-800=move || current_page.get() == UserPage::Settings
                                                                                class:text-white=move || current_page.get() == UserPage::Settings
                                                                                class:text-gray-400=move || current_page.get() != UserPage::Settings
                                                                                class:hover:text-white=move || current_page.get() != UserPage::Settings
                                                                                class:hover:bg-gray-800=move || current_page.get() != UserPage::Settings
                                                                            >
                                                                                <svg
                                                                                    class="size-6 shrink-0"
                                                                                    class:text-white=move || current_page.get() == UserPage::Settings
                                                                                    class:text-gray-400=move || current_page.get() != UserPage::Settings
                                                                                    class:group-hover:text-white=move || current_page.get() != UserPage::Settings
                                                                                    fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true"
                                                                                >
                                                                                  <path stroke-linecap="round" stroke-linejoin="round" d=UserPage::Settings.svg()></path>
                                                                                </svg>
                                                                                {UserPage::Settings.label()}
                                                                            </A>
                                                                        </li>
                                                                         <li>
                                                                            <ActionForm action=logout>
                                                                                <button
                                                                                    type="submit"
                                                                                    class="group flex w-full gap-x-3 rounded-md p-2 text-sm/6 font-semibold text-gray-400 hover:bg-gray-800 hover:text-white"
                                                                                >
                                                                                    <svg
                                                                                        class="size-6 shrink-0 text-gray-400 group-hover:text-white"
                                                                                        fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true"
                                                                                    >
                                                                                      <path stroke-linecap="round" stroke-linejoin="round" d=logout_icon_path.clone()></path>
                                                                                    </svg>
                                                                                    "Log Out"
                                                                                </button>
                                                                            </ActionForm>
                                                                        </li>
                                                                    </div>
                                                                </ul>
                                                            </nav>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>


                                            <div class="hidden lg:fixed lg:inset-y-0 lg:z-50 lg:flex lg:w-72 lg:flex-col">
                                                <div class="flex grow flex-col gap-y-5 overflow-y-auto bg-gray-950 px-6 pb-4">
                                                    <div class="flex h-16 shrink-0 items-center">
                                                        <A href="/">
                                                         <img class="h-8 w-auto" src="/favicon.ico" alt="Your Company"/> // Update logo path
                                                            </A>
                                                    </div>
                                                    <nav class="flex flex-1 flex-col">
                                                        <ul role="list" class="flex flex-1 flex-col gap-y-7">
                                                            <li>
                                                                <ul role="list" class="-mx-2 space-y-1">
                                                                     <NavItem page=UserPage::Dashboard current_page=current_page/>
                                                                     <NavItem page=UserPage::Projects current_page=current_page/>
                                                                </ul>
                                                            </li>


                                                            <div class="mt-auto -mx-2 space-y-1">
                                                                <li>
                                                                     <A
                                                                        href=UserPage::Settings.href()
                                                                        attr:class="group flex gap-x-3 rounded-md p-2 text-sm/6 font-semibold"
                                                                        class:bg-gray-800=move || current_page.get() == UserPage::Settings
                                                                        class:text-white=move || current_page.get() == UserPage::Settings
                                                                        class:text-gray-400=move || current_page.get() != UserPage::Settings
                                                                        class:hover:text-white=move || current_page.get() != UserPage::Settings
                                                                        class:hover:bg-gray-800=move || current_page.get() != UserPage::Settings
                                                                    >
                                                                        <svg
                                                                            class="size-6 shrink-0"
                                                                            class:text-white=move || current_page.get() == UserPage::Settings
                                                                            class:text-gray-400=move || current_page.get() != UserPage::Settings
                                                                            class:group-hover:text-white=move || current_page.get() != UserPage::Settings
                                                                            fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true"
                                                                        >
                                                                            <path stroke-linecap="round" stroke-linejoin="round" d=UserPage::Settings.svg()></path>
                                                                        </svg>
                                                                        {UserPage::Settings.label()}
                                                                    </A>
                                                                </li>
                                                                <li> // Logout Item
                                                                    <ActionForm action=logout>
                                                                        <button
                                                                            type="submit"
                                                                            class="group cursor-pointer flex w-full gap-x-3 rounded-md p-2 text-sm/6 font-semibold text-gray-400 hover:bg-gray-800 hover:text-white"
                                                                        >
                                                                            <svg
                                                                                class="size-6 shrink-0 text-gray-400 group-hover:text-white"
                                                                                fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true"
                                                                            >
                                                                              <path stroke-linecap="round" stroke-linejoin="round" d=logout_icon_path2.clone()></path>
                                                                            </svg>
                                                                            "Log Out"
                                                                        </button>
                                                                    </ActionForm>
                                                                </li>
                                                            </div>
                                                        </ul>
                                                    </nav>
                                                </div>
                                            </div>

                                            <div class="lg:pl-72 h-full">
                                                <div class="sticky top-0 z-40 flex h-16 shrink-0 items-center gap-x-4 border-b border-gray-700 bg-gray-950 px-4 shadow-sm sm:gap-x-6 sm:px-6 lg:hidden">
                                                    <button
                                                        type="button"
                                                        class="-m-2.5 p-2.5 text-gray-400 lg:hidden"
                                                        on:click=move |_| set_mobile_sidebar_open(true)
                                                    >
                                                        <span class="sr-only">"Open sidebar"</span>
                                                        <svg class="size-6" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true">
                                                           <path stroke-linecap="round" stroke-linejoin="round" d=menu_icon_path></path>
                                                        </svg>
                                                    </button>
                                                </div>
                                                <div class="bg-gray-900 text-white p-4 sm:p-6 lg:p-8 h-full">
                                                    <div class=" h-full">
                                                        <Outlet/>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    },
                                )
                            }
                            _ => Either::Left(()),
                        }
                    })
            }}
        </Suspense>
    }
}

#[component]
fn NavItem(
    #[prop(into)] page: UserPage,
    #[prop(into)] current_page: ReadSignal<UserPage>,
) -> impl IntoView {
    view! {
        <li>
            <A
                href=page.href()
                attr:class=move || {
                    format!(
                        "sidebar-link {}",
                        if current_page() == page {
                            "sidebar-link-active"
                        } else {
                            "sidebar-link-inactive"
                        },
                    )
                }
            >
                <svg
                    class=move || {
                        format!(
                            "sidebar-link-svg {}",
                            if current_page() == page {
                                "sidebar-link-svg-active"
                            } else {
                                "sidebar-link-svg-inactive"
                            },
                        )
                    }

                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="1.5"
                    stroke="currentColor"
                    aria-hidden="true"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" d=page.svg()></path>
                </svg>
                {page.label()}
            </A>
        </li>
    }
}
