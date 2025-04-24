use leptos::prelude::{CustomAttribute, Set};
use std::str::FromStr;
use crate::app::IntoView;
use common::hosting_action::HostingAction;
use common::permission::Permission;
use common::{ProjectSlug, ProjectSlugStr, UserSlug};
use leptos::prelude::{Get, GlobalAttributes, Show};
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::{expect_context, Action, ElementChild, ServerFnError, Signal};
use leptos::prelude::{signal, ClassAttribute, OnAttribute, Resource, Suspend, Suspense};
use leptos::{component, server, view};
use common::server_action::user_action::UserAction;
use crate::app::pages::user::projects::project::{get_project, MemoProjectParams};
use crate::app::pages::CsrfValue;
use crate::models::Project;

#[component]
pub fn ProjectSettings() -> impl IntoView {
    let params: MemoProjectParams = expect_context();
    //let project:Project = expect_context();

    let project_resource = Resource::new(
        move || params.get().unwrap().project_slug,
        move |project_slug| get_project(project_slug),
    );
    let toggle_project_action = Action::new(
        |intput :&(ProjectSlugStr, String, bool)| {
            let (project_slug, csrf, is_active) = intput.clone();
            async move {
                toggle_project_active(csrf, project_slug, is_active).await
            }
        },
    );
    let delete_project_action = Action::new(
        |intput :&(ProjectSlugStr, String)| {
            let (project_slug, csrf) = intput.clone();
            async move {
                delete_project(csrf, project_slug).await
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

    let project_data = move || {
        project_resource
            .get()
            .map(|p| p.unwrap_or_default())
            .unwrap_or_default()
    };
    let (preview_version, set_preview_version) = signal(0u32);
    let slug = Signal::derive(move || params.get().unwrap().project_slug.clone());

    let csrf_value = expect_context::<Signal<CsrfValue>>();

    view! {
        <div class="space-y-10">
             <Suspense fallback=move || view!{Loading...}>
                {move || {
                    Suspend::new(async move {
                        let(hosting_url, project) = project_data();
                        let (is_active, set_is_active) = signal(project.is_active);
                        let (hosting_url_signal, set_hosting_url_signal) = signal(hosting_url);

                        let on_toggle_project = move |_| {
                            let project_slug = slug.get();
                            let csrf = csrf_value.get().0.clone();
                            set_is_active(!is_active.get());
                            toggle_project_action.dispatch((project_slug,csrf, is_active.get()));
                        };
                        let on_reload_project = move |_| {
                            let project_slug = slug.get();
                            let csrf = csrf_value.get().0.clone();
                            reload_project_action.dispatch((project_slug, csrf));
                        };

                        let on_delete_project = move |_| {
                            let project_slug = slug.get();
                            let csrf = csrf_value.get().0.clone();
                            let confirmed = if let Some(window) = web_sys::window() {
                                window
                                    .confirm_with_message(
                                        &format!(
                                            "Are you sure you want to delete the project '{}'?",project_slug,
                                        ),
                                    )
                                    .unwrap_or(false)
                            } else {
                                false
                            };
                             if !confirmed {
                                return;
                            }
                            delete_project_action.dispatch((project_slug, csrf));
                        };

                         let refresh_preview = move || {
                            set_preview_version(preview_version() + 1);
                            // Or with timestamp:
                            // set_preview_version.set(Utc::now().timestamp_millis());
                        };

                        view!{
                             <div class="section-border">
                                    <h2 class="section-title">"Project Status & Activation"</h2>
                                    <p class="section-desc">"Control whether your project is live and accessible."</p>
                                    <div class="mt-6 flex items-center justify-between">
                                        <div>
                                            <span class="text-sm font-medium text-white">"Current Status: "</span>
                                            <span class=move || format!("text-sm font-semibold {}", if is_active() { "text-green-400" } else { "text-gray-500" })>
                                                {move || if is_active() { "Online" } else { "Offline" }}
                                            </span>
                                        </div>
                                        <button
                                            class="btn btn-primary"
                                            on:click=on_toggle_project
                                            disabled=move || toggle_project_action.pending().get()
                                        >
                                            {move || if is_active() { "Deactivate" } else { "Activate" }}
                                        </button>
                                    </div>
                                    // Feedback for toggle action
                                    <div class="mt-2 text-sm text-right min-h-[1.25em]"> // Reserve space for feedback
                                        <Show when=move || toggle_project_action.pending().get()>
                                            <p class="text-gray-400">"Updating status..."</p>
                                        </Show>
                                        {move || toggle_project_action.value().get().map(|result| {
                                            match result {
                                                Ok(_) => view! { <p class="text-green-400">"Status updated successfully."</p> }.into_view(),
                                                Err(e) => view! { <p class="text-red-400">{format!("Error: {}", e)}</p> }.into_view(),
                                            }
                                        })}
                                    </div>
                                    <Show when=move || is_active()>
                                    <div class="mt-6 pt-6 border-t border-gray-700 space-y-4">
                                        <div class="flex justify-between items-center"> // Container for title and refresh button
                                            <h3 class="text-base font-semibold leading-6 text-white">"Live Preview & Link"</h3>
                                            // --- Manual Refresh Button ---
                                            <button
                                                class="btn btn-secondary" // Adjust style as needed
                                                on:click=move |_| refresh_preview() // Call the refresh function
                                            >
                                                 // Optional: Refresh Icon
                                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 mr-1"> <path fill-rule="evenodd" d="M15.312 11.424a5.5 5.5 0 0 1-9.201-4.42 1.75 1.75 0 1 1 2.971 1.506 2.001 2.001 0 0 0 3.26 1.415 1.75 1.75 0 1 1 2.97 1.5Z" clip-rule="evenodd" /><path d="M4.688 8.576a5.5 5.5 0 0 1 9.201 4.42 1.75 1.75 0 1 1-2.971-1.506 2.001 2.001 0 0 0-3.26-1.415 1.75 1.75 0 1 1-2.97-1.5Z" /></svg>
                                                "Refresh Preview"
                                            </button>
                                        </div>
                                        <div>
                                            <a class="inline-flex items-center gap-x-1.5 rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 transition duration-150 ease-in-out"
                                               href=move || format!("http://{}.{}/", ProjectSlug::from_str(&slug.get()).unwrap().to_unix(), hosting_url_signal.get())
                                               target="_blank" rel="noopener noreferrer">
                                                "View Live Project"
                                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4" aria-hidden="true"><path fill-rule="evenodd" d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .414.336.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h4a.75.75 0 0 1 0 1.5h-4Zm6.5-1.75a.75.75 0 0 0 0-1.5h4.5a.75.75 0 0 0 .75-.75V1a.75.75 0 0 0-1.5 0v3.75h-3.75a.75.75 0 0 0-.75.75Z" clip-rule="evenodd" /></svg>
                                            </a>
                                        </div>
                                        <iframe
                                            class="mt-4 w-full h-80 border border-gray-600 rounded-lg bg-gray-800 shadow-inner"
                                            // --- Append Cache Buster to src ---
                                            src=move || format!(
                                                "http://{}.{}/?_cb={}", // Added query parameter `_cb`
                                                ProjectSlug::from_str(&slug.get()).unwrap().to_unix(),
                                                hosting_url_signal.get(),
                                                preview_version.get() // Use the signal value
                                            )
                                            title={format!("Live preview for project: {}", slug.get())}
                                            // sandbox=MaybeSignal::Static("allow-scripts allow-same-origin".to_string()) // Example sandbox prop
                                        >
                                            "Your browser does not support iframes."
                                        </iframe>
                                    </div>
                                </Show>
                                </div>

                                // --- Reload Section ---
                                <div class="section-border">
                                    <h2 class="section-title">"Reload Project Cache"</h2>
                                    <p class="section-desc">"Force the hosting server to reload your project files from disk. Use this after manual SFTP uploads if changes aren't reflected."</p>
                                     <div class="mt-6 flex items-center justify-end"> // Aligned to the right
                                        <button
                                            class="btn btn-primary"
                                            on:click=on_reload_project
                                            disabled=move || reload_project_action.pending().get()
                                        >
                                            "Reload Project"
                                        </button>
                                    </div>
                                     // Feedback for reload action
                                    <div class="mt-2 text-sm text-right min-h-[1.25em]">
                                        <Show when=move || reload_project_action.pending().get()>
                                            <p class="text-gray-400">"Reloading project..."</p>
                                        </Show>
                                        {move || reload_project_action.value().get().map(|result| {
                                            match result {
                                                Ok(_) => view! { <p class="text-green-400">"Project reload requested."</p> }.into_view(),
                                                Err(e) => view! { <p class="text-red-400">{format!("Error: {}", e)}</p> }.into_view(),
                                            }
                                        })}
                                    </div>
                                </div>

                                // --- Danger Zone ---
                                <div class="section-border border-red-500/30 pb-6"> // Subtle red border hint
                                     <h2 class="section-title text-red-400">"Danger Zone"</h2> // Red title
                                     <p class="section-desc">"These actions are permanent and cannot be undone."</p>
                                     <div class="mt-6 flex items-center justify-between">
                                        <div>
                                            <p class="text-sm font-medium text-white">"Delete this project"</p>
                                            <p class="text-xs text-gray-400">"All associated data and files will be permanently removed."</p>
                                        </div>
                                         <button
                                             class="btn btn-danger"
                                             on:click=on_delete_project
                                             disabled=move || delete_project_action.pending().get()
                                         >
                                             "Delete Project"
                                         </button>
                                    </div>
                                    // Feedback for delete action
                                     <div class="mt-2 text-sm text-right min-h-[1.25em]">
                                        <Show when=move || delete_project_action.pending().get()>
                                            <p class="text-gray-400">"Deleting project..."</p>
                                        </Show>
                                        {move || delete_project_action.value().get().map(|result| {
                                            match result {
                                                 // Usually redirects before success message shows
                                                Ok(_) => view! { <p class="text-green-400">"Project deleted."</p> }.into_view(),
                                                Err(e) => view! { <p class="text-red-400">{format!("Error: {}", e)}</p> }.into_view(),
                                            }
                                        })}
                                    </div>
                                </div>
                        }

                    })
                }}
            </Suspense>
        </div>
    }
}


#[server]
pub async fn delete_project(
    csrf: String,
    project_slug: ProjectSlugStr,
) -> Result<(), ServerFnError>{
    use crate::api::ssr::request_hosting_action;
    use crate::security::permission::ssr::handle_project_permission_request;
    use crate::api::ssr::{request_server_action, request_server_project_action};

    handle_project_permission_request(
        project_slug,
        Permission::Owner,
        Some(csrf),
        |auth, db, project_slug| async move {
            let user_ids = sqlx::query!(
                "DELETE FROM permissions WHERE project_id = $1 RETURNING user_id",
                project_slug.id,
            )
                .fetch_all(&db)
                .await?;
            let users = sqlx::query!(
                "SELECT id,username FROM users WHERE id = ANY($1)",
                &user_ids.iter().map(|u| u.user_id).collect::<Vec<_>>()
            )
                .fetch_all(&db)
                .await?;
            let user_slugs = users.into_iter()
                .map(|u| UserSlug::new(u.id, u.username))
                .collect::<Vec<_>>();
            let is_active = sqlx::query!(
                "delete from projects where id = $1 returning is_active",
                project_slug.id
            )
            .fetch_one(&db)
            .await?.is_active;

            if is_active{
                let action = HostingAction::StopServingProject;
                request_hosting_action(project_slug.clone(), action).await?;
            }

            request_server_action(
                UserAction::RemoveProject {
                    user_slugs,
                    project_slug,
                }.into()
            ).await?;
            leptos_axum::redirect("/user/projects");

            Ok(())
        },
    )
    .await
}

#[server]
pub async fn toggle_project_active(
    csrf: String,
    project_slug: ProjectSlugStr,
    is_active: bool,
) -> Result<(), ServerFnError> {
    use crate::api::ssr::request_hosting_action;
    use crate::security::permission::ssr::handle_project_permission_request;
    handle_project_permission_request(
        project_slug,
        Permission::Owner,
        Some(csrf),
        |_, db, project_slug| async move {
            let project = sqlx::query!(
                "UPDATE projects SET is_active = $1 WHERE id = $2",
                is_active,
                project_slug.id
            )
            .execute(&db)
            .await?;
            let action = if is_active {
                HostingAction::ServeReloadProject
            } else {
                HostingAction::StopServingProject
            };
            request_hosting_action(project_slug, action).await?;
            Ok(())
        },
    )
    .await
}

#[server]
pub async fn on_reload_project(
    csrf: String,
    project_slug: ProjectSlugStr,
) -> Result<(), ServerFnError> {
    use crate::api::ssr::request_hosting_action;
    use crate::security::permission::ssr::handle_project_permission_request;
    handle_project_permission_request(
        project_slug,
        Permission::Owner,
        Some(csrf),
        |_, _, project_slug| async move {
            let action = HostingAction::ServeReloadProject;
            request_hosting_action(project_slug, action).await?;
            Ok(())
        },
    )
    .await
}
