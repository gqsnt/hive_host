use crate::app::pages::user::projects::project::{ProjectSlugSignal, ProjectUpdateSignal};
use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use crate::app::{commit_display, IntoView};
use common::{ProjectSlugStr, ServerId};
use leptos::prelude::{IntoMaybeErased, NodeRef, NodeRefAttribute, Read, Resource, RwSignal, Set, Suspend, Suspense, Transition, Update, Write};
use leptos::prelude::{expect_context, Action, ElementChild, Signal};
use leptos::prelude::{signal, ClassAttribute, OnAttribute};
use leptos::prelude::{CustomAttribute, Effect};
use leptos::prelude::{Get, GlobalAttributes, Show};
use leptos::{component, view};
use leptos::either::{EitherOf3, EitherOf4};
use leptos::html::{Input, Select};
use reactive_stores::Store;
use crate::app::pages::user::projects::new_project::server_fns::get_github_repo_branches;




#[component]
pub fn ProjectSettings() -> impl IntoView {
    let global_state: Store<GlobalState> = expect_context();
    let project_slug_signal: Signal<ProjectSlugSignal> = expect_context();
    let project_update_signal : RwSignal<ProjectUpdateSignal> = expect_context();
    let slug = move ||
        project_slug_signal.get().0;


    let csrf = move || {
        global_state.csrf().get().unwrap_or_default()
    };


    let (preview_version, set_preview_version) = signal(0u32);

    let refresh_preview = move || {
        set_preview_version(preview_version() + 1);
    };

    let delete_project_action = Action::new(|intput: &(ServerId, ProjectSlugStr, String)| {
        let (server_id, project_slug, csrf) = intput.clone();
        async move { server_fns::delete_project(csrf, server_id, project_slug).await }
    });

    let sync_dev_action = Action::new(|input: &(String, ServerId, ProjectSlugStr)| {
        let (csrf, sid, slug) = input.clone();
        async move { server_fns::sync_development_action(csrf, sid, slug).await }
    });

    let deploy_prod_action = Action::new(|input: &(String, ServerId, ProjectSlugStr, bool)| {
        let (csrf, sid, slug, auto_deploy) = input.clone();
        async move { server_fns::deploy_auto_git_to_production(csrf, sid, slug, auto_deploy).await }
    });

    let update_branch_action = Action::new(|input: &(String, ServerId, ProjectSlugStr, String, String)| {
        let (csrf, sid, slug, branch_name, branch_commit) = input.clone();
        async move { server_fns::update_default_branch_action(csrf, sid, slug, branch_name, branch_commit).await }
    });

    let (delete_project_action_result, set_delete_project_action_result) = signal("".to_string());
    Effect::new(move |_| {
        let result = delete_project_action.value().get();
        if let Some(Ok(_)) = result {
            set_delete_project_action_result("Project deleted".to_string());
        } else if let Some(Err(e)) = result {
            set_delete_project_action_result(format!("Error: {e}"));
        }
    });


    view! {
        <div class="space-y-10">
            <Transition fallback=move || {
                view! { "Loading ..." }
            }>
                {move || Suspend::new(async move {
                    let server_id = move || {
                        global_state
                            .project()
                            .read()
                            .as_ref()
                            .map(|p| p.project.server_id)
                            .unwrap_or_default()
                    };
                    let has_git_projet = Signal::derive(move || {
                        global_state
                            .project()
                            .read()
                            .as_ref()
                            .map(|inner| inner.project.git_project.is_some())
                            .unwrap_or_default()
                    });
                    let is_active = Signal::derive(move || {
                        global_state
                            .project()
                            .read()
                            .as_ref()
                            .and_then(|inner| inner.project.active_snapshot_id)
                            .is_some()
                    });
                    let permission_signal = Signal::derive(move || {
                        global_state
                            .project()
                            .read()
                            .as_ref()
                            .map(|p| p.permission)
                            .unwrap_or_default()
                    });
                    let hosting_url = Signal::derive(move || {
                        global_state
                            .project()
                            .read()
                            .as_ref()
                            .map(|p| p.project.hosting_address.clone())
                            .unwrap_or_default()
                    });
                    let on_delete_project = move |_| {
                        let project_slug = slug();
                        let confirmed = web_sys::window()
                            .map(|window| {
                                window
                                    .confirm_with_message(
                                        &format!(
                                            "Are you sure you want to delete the project '{project_slug}'?",
                                        ),
                                    )
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);
                        if !confirmed {
                            return;
                        }
                        delete_project_action.dispatch((server_id(), project_slug, csrf()));
                    };

                    view! {
                        <Show
                            when=move || has_git_projet()
                            fallback=|| {
                                view! {
                                    <div class="text-gray-400">
                                        "Project is not linked to GitHub."
                                    </div>
                                }
                            }
                        >
                            {
                                let current_prod_commit = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .and_then(|g| {
                                                    g.prod_branch_commit.as_ref().map(|prod| prod.1.clone())
                                                })
                                        })
                                };
                                let current_prod_branch = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .and_then(|g| {
                                                    g.prod_branch_commit.as_ref().map(|prod| prod.0.clone())
                                                })
                                        })
                                };
                                let current_dev_commit = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .map(|g| g.dev_commit.clone())
                                        })
                                        .unwrap_or_default()
                                };
                                let last_commit = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .map(|g| g.last_commit.clone())
                                        })
                                        .unwrap_or_default()
                                };
                                let repo_full_name = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .map(|g| g.repo_full_name.clone())
                                        })
                                        .unwrap_or_default()
                                };
                                let branch_name = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .map(|g| g.branch_name.clone())
                                        })
                                        .unwrap_or_default()
                                };
                                let is_auto_deploy = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner.project.git_project.as_ref().map(|g| g.auto_deploy)
                                        })
                                        .unwrap_or_default()
                                };
                                let installation_id = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .map(|g| g.installation_id)
                                        })
                                        .unwrap_or_default()
                                };
                                let user_githubs_id = move || {
                                    global_state
                                        .project()
                                        .read()
                                        .as_ref()
                                        .and_then(|inner| {
                                            inner
                                                .project
                                                .git_project
                                                .as_ref()
                                                .map(|g| g.user_githubs_id)
                                        })
                                        .unwrap_or_default()
                                };
                                let branches_resource = Resource::new_bincode(
                                    move || { (csrf(), user_githubs_id(), repo_full_name()) },
                                    |(csrf, user_githubs_id, repo)| {
                                        get_github_repo_branches(csrf, Some(user_githubs_id), repo)
                                    },
                                );
                                let prod_is_behind = move || {
                                    let prod_commit = current_prod_commit();
                                    prod_commit.is_none()
                                        || !prod_commit.unwrap().eq(&last_commit())
                                };
                                let dev_is_behind = move || {
                                    !current_dev_commit().eq(&last_commit())
                                };
                                let can_deploy_to_prod = move || {
                                    !current_dev_commit()
                                        .eq(&current_prod_commit().unwrap_or_default())
                                };
                                let delete_untracked_ref = NodeRef::<Input>::new();

                                view! {
                                    <div class="section-border">
                                        <h2 class="section-title">"Project GitHub Repository"</h2>
                                        <p class="section-desc">
                                            {format!(
                                                "Linked to: {} (Branch: {})",
                                                repo_full_name(),
                                                branch_name(),
                                            )}
                                        </p>
                                        <div class="mt-6 space-y-4">
                                            <div class="flex items-center justify-between p-3 bg-gray-800 rounded-md">
                                                <div>
                                                    <p class="font-medium text-white">
                                                        {move || {
                                                            format!(
                                                                "Production Status{}",
                                                                current_prod_branch()
                                                                    .map(|b| format!(" (Branch: {})", b))
                                                                    .unwrap_or_default(),
                                                            )
                                                        }}
                                                    </p>
                                                    {move || match (
                                                        prod_is_behind(),
                                                        current_prod_commit().is_some(),
                                                    ) {
                                                        (_, false) => {
                                                            EitherOf3::B(
                                                                view! {
                                                                    <p class="text-sm text-gray-400">
                                                                        "Not yet deployed from Git."
                                                                    </p>
                                                                },
                                                            )
                                                        }
                                                        (false, true) => {
                                                            EitherOf3::A(
                                                                view! {
                                                                    <p class="text-sm text-green-400">
                                                                        {format!(
                                                                            "Up-to-date (Commit: {})",
                                                                            commit_display(&current_prod_commit().unwrap_or_default()),
                                                                        )}
                                                                    </p>
                                                                },
                                                            )
                                                        }
                                                        (true, true) => {
                                                            EitherOf3::C(
                                                                view! {
                                                                    <p class="text-sm text-yellow-400">
                                                                        {format!(
                                                                            "Behind. Current: {}, Latest: {}",
                                                                            commit_display(&current_prod_commit().unwrap_or_default()),
                                                                            commit_display(&last_commit()),
                                                                        )}
                                                                    </p>
                                                                },
                                                            )
                                                        }
                                                    }}
                                                </div>
                                                <button
                                                    class="btn btn-success"
                                                    on:click=move |_| {
                                                        deploy_prod_action
                                                            .dispatch((csrf(), server_id(), slug(), !is_auto_deploy()));
                                                        project_update_signal
                                                            .write_only()
                                                            .set(project_update_signal.read_only().get().tick());
                                                    }
                                                    disabled=deploy_prod_action.pending().get()
                                                >
                                                    {if deploy_prod_action.pending().get() {
                                                        if is_auto_deploy() {
                                                            "Disabling Auto Deploy..."
                                                        } else {
                                                            "Enabling Auto Deploy..."
                                                        }
                                                    } else {
                                                        if is_auto_deploy() {
                                                            "Disable Auto Deploy"
                                                        } else {
                                                            "Enable Auto Deploy"
                                                        }
                                                    }}
                                                </button>
                                            </div>

                                            // Development Status
                                            <div class="flex items-center justify-between p-3 bg-gray-800 rounded-md">
                                                <div>
                                                    <p class="font-medium text-white">Development Status</p>
                                                    <Show
                                                        when=move || dev_is_behind()
                                                        fallback=move || {
                                                            view! {
                                                                <p class="text-sm text-green-400">
                                                                    {format!(
                                                                        "Up-to-date (Commit: {})",
                                                                        commit_display(&current_dev_commit()),
                                                                    )}
                                                                </p>
                                                            }
                                                        }
                                                    >
                                                        <p class="text-sm text-yellow-400">
                                                            {format!(
                                                                "Behind. Synced: {}, Latest: {}",
                                                                commit_display(&current_dev_commit()),
                                                                commit_display(&last_commit()),
                                                            )}
                                                        </p>
                                                    </Show>
                                                </div>
                                                <Show when=move || dev_is_behind()>
                                                    <div class="flex space-x-2">
                                                        <button
                                                            class="btn btn-secondary"
                                                            on:click=move |_| {
                                                                sync_dev_action.dispatch((csrf(), server_id(), slug()));
                                                                project_update_signal
                                                                    .write_only()
                                                                    .set(project_update_signal.read_only().get().tick());
                                                            }
                                                            disabled=sync_dev_action.pending().get()
                                                        >
                                                            {if sync_dev_action.pending().get() {
                                                                "Syncing..."
                                                            } else {
                                                                "Sync Development"
                                                            }}
                                                        </button>
                                                    </div>
                                                </Show>
                                            </div>

                                            // Change Default Branch
                                            <div class="p-3 bg-gray-800 rounded-md">
                                                <p class="form-label">"Change Default Branch"</p>
                                                <Transition fallback=move || {
                                                    view! {
                                                        <p class="text-sm text-gray-400">"Loading branches..."</p>
                                                    }
                                                }>
                                                    {move || Suspend::new(async move {
                                                        let branches = branches_resource.try_get().flatten();
                                                        match branches {
                                                            Some(Ok(branches)) => {
                                                                if branches.is_empty() {
                                                                    EitherOf4::A(
                                                                        view! {
                                                                            <p class="text-sm text-yellow-400">
                                                                                "No other branches found."
                                                                            </p>
                                                                        },
                                                                    )
                                                                } else {
                                                                    let branch_select_ref = NodeRef::<Select>::new();
                                                                    EitherOf4::B(
                                                                        view! {
                                                                            <div class="flex items-end space-x-2 mt-2">
                                                                                <select
                                                                                    class="form-select flex-grow"
                                                                                    node_ref=branch_select_ref
                                                                                >
                                                                                    <option value="">"-- Select new branch --"</option>
                                                                                    {branches
                                                                                        .into_iter()
                                                                                        .filter(|b| b.name != branch_name())
                                                                                        .map(|branch| {
                                                                                            // Don't show current branch as an option to switch to
                                                                                            view! {
                                                                                                <option value=format!(
                                                                                                    "{}={}",
                                                                                                    branch.name.clone(),
                                                                                                    branch.commit,
                                                                                                )>{branch.name.clone()}</option>
                                                                                            }
                                                                                        })
                                                                                        .collect::<Vec<_>>()}
                                                                                </select>
                                                                                <button
                                                                                    class="btn btn-warning min-w-[8rem]"
                                                                                    on:click=move |_| {
                                                                                        if let Some(select_el) = branch_select_ref.get() {
                                                                                            let val = select_el.value();
                                                                                            if !val.is_empty() {
                                                                                                let parts: Vec<&str> = val.split('=').collect();
                                                                                                if parts.len() == 2 {
                                                                                                    let branch_name = parts[0].to_string();
                                                                                                    let branch_commit = parts[1].to_string();
                                                                                                    update_branch_action
                                                                                                        .dispatch((
                                                                                                            csrf(),
                                                                                                            server_id(),
                                                                                                            slug(),
                                                                                                            branch_name.clone(),
                                                                                                            branch_commit.clone(),
                                                                                                        ));
                                                                                                    project_update_signal
                                                                                                        .write_only()
                                                                                                        .set(project_update_signal.read_only().get().tick());
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    disabled=update_branch_action.pending().get()
                                                                                >
                                                                                    {if update_branch_action.pending().get() {
                                                                                        "Updating..."
                                                                                    } else {
                                                                                        "Update Branch"
                                                                                    }}
                                                                                </button>
                                                                            </div>
                                                                        },
                                                                    )
                                                                }
                                                            }
                                                            Some(Err(e)) => {
                                                                EitherOf4::C(
                                                                    view! {
                                                                        <p class="text-sm text-red-400">
                                                                            {format!("Error loading branches: {e}")}
                                                                        </p>
                                                                    },
                                                                )
                                                            }
                                                            None => {
                                                                EitherOf4::D(
                                                                    view! {
                                                                        <p class="text-sm text-gray-400">"Loading branches..."</p>
                                                                    },
                                                                )
                                                            }
                                                        }
                                                    })}
                                                </Transition>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }
                        </Show>
                        <div class="section-border">
                            <h2 class="section-title">"Project Status & Activation"</h2>
                            <p class="section-desc">
                                "Control whether your project is live and accessible."
                            </p>
                            <Show
                                when=move || is_active()
                                fallback=move || {
                                    view! {
                                        <div class="flex items-center my-2">
                                            <p class="text-sm font-medium text-white">
                                                "Project is not live"
                                            </p>
                                            <p class="text-xs text-gray-400 ml-4">
                                                "Set a Snapshot as active to make it live."
                                            </p>
                                        </div>
                                    }
                                }
                            >

                                <div class="mt-6 pt-6 border-t border-gray-700 space-y-4">
                                    <div class="flex justify-between items-center">
                                        <h3 class="text-base font-semibold leading-6 text-white">
                                            "Live Preview & Link"
                                        </h3>
                                        <button
                                            class="btn btn-secondary"
                                            on:click=move |_| refresh_preview()
                                        >
                                            // Optional: Refresh Icon
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 20 20"
                                                fill="currentColor"
                                                class="w-4 h-4 mr-1"
                                            >
                                                <path
                                                    fill-rule="evenodd"
                                                    d="M15.312 11.424a5.5 5.5 0 0 1-9.201-4.42 1.75 1.75 0 1 1 2.971 1.506 2.001 2.001 0 0 0 3.26 1.415 1.75 1.75 0 1 1 2.97 1.5Z"
                                                    clip-rule="evenodd"
                                                />
                                                <path d="M4.688 8.576a5.5 5.5 0 0 1 9.201 4.42 1.75 1.75 0 1 1-2.971-1.506 2.001 2.001 0 0 0-3.26-1.415 1.75 1.75 0 1 1-2.97-1.5Z" />
                                            </svg>
                                            "Refresh Preview"
                                        </button>
                                    </div>
                                    <div>
                                        <a
                                            class="inline-flex items-center gap-x-1.5 rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 transition duration-150 ease-in-out"

                                            href=move || {
                                                format!("http://{}.{}/", slug(), hosting_url())
                                            }
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        >
                                            "View Live Project"
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 20 20"
                                                fill="currentColor"
                                                class="w-4 h-4"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    fill-rule="evenodd"
                                                    d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .414.336.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h4a.75.75 0 0 1 0 1.5h-4Zm6.5-1.75a.75.75 0 0 0 0-1.5h4.5a.75.75 0 0 0 .75-.75V1a.75.75 0 0 0-1.5 0v3.75h-3.75a.75.75 0 0 0-.75.75Z"
                                                    clip-rule="evenodd"
                                                />
                                            </svg>
                                        </a>
                                    </div>
                                    <iframe
                                        class="mt-4 w-full h-80 border border-gray-600 rounded-lg bg-gray-800 shadow-inner"
                                        src=move || {
                                            format!(
                                                "http://{}.{}/?_cb={}",
                                                slug(),
                                                hosting_url(),
                                                preview_version.get(),
                                            )
                                        }
                                        title=format!("Live preview for project: {}", slug())
                                    />
                                </div>
                            </Show>

                        </div>

                        // --- Danger Zone ---
                        // Subtle red border hint
                        <div class="pb-6" class=("hidden", move || !permission_signal().is_owner())>
                            // Red title
                            <h2 class="section-title text-red-400">"Danger Zone"</h2>
                            <p class="section-desc">
                                "These actions are permanent and cannot be undone."
                            </p>
                            <div class="mt-6 flex items-center justify-between">
                                <div>
                                    <p class="text-sm font-medium text-white">
                                        "Delete this project"
                                    </p>
                                    <p class="text-xs text-gray-400">
                                        "All associated data and files will be permanently removed."
                                    </p>
                                </div>
                                <button
                                    class="btn btn-danger"
                                    on:click=on_delete_project
                                    disabled=move || delete_project_action.pending().get()
                                >
                                    "Delete Project"
                                </button>
                            </div>
                            <div class="mt-2 text-sm text-right min-h-[1.25em]">
                                {delete_project_action_result}
                            </div>
                        </div>
                    }
                })}
            </Transition>
        </div>
    }
}

pub mod server_fns {
    use leptos::logging::log;
    use crate::AppResult;
    use common::{ProjectSlugStr, ServerId};
    use leptos::server;
    use leptos::server_fn::codec::Bincode;
    use common::server_action::project_action::git_action::ProjectGitAction;
    use crate::app::pages::user::projects::new_project::server_fns::get_github_repo_branches;
    use crate::models::GitProject;


    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::api::ssr::request_server_project_action;
        use crate::security::permission::ssr::handle_project_permission_request;
        use crate::api::ssr::{request_user_action};
        use common::server_action::permission::Permission;
        use common::Slug;
        use common::server_action::project_action::snapshot::ProjectSnapshotAction;
        use common::server_action::user_action::ServerUserAction;
            use crate::ssr::ws_clients;
    }}

    #[server(input=Bincode, output=Bincode)]
    pub async fn sync_development_action(
        csrf: String,
        server_id: ServerId,
        project_slug: ProjectSlugStr,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Write,
            Some(csrf.clone()),
            move |_, db, proj_slug| async move {
                let project_git = sqlx::query!(
                    "SELECT p.id as project_id,  pgi.id as project_github_id, pgi.last_commit as last_commit, pgi.branch_name as branch_name  FROM projects p left join projects_github pgi on p.project_github_id = pgi.id  WHERE p.id = $1",
                    proj_slug.id
                )
                    .fetch_one(&db)
                    .await?;
                ssr::inner_update_dev_with_git(
                    &db,
                    ws_clients()?,
                    server_id,
                    proj_slug,
                    project_git.project_github_id,
                    project_git.branch_name,
                    project_git.last_commit,
                ).await?;

                
                Ok(())
            },
        )
            .await
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn deploy_auto_git_to_production(
        csrf: String,
        server_id: ServerId,
        project_slug: ProjectSlugStr,
        auto_deploy: bool,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner,
            Some(csrf),
            move |auth_user_id, db, proj_slug| async move {
                let project_git = sqlx::query!(
                    "SELECT
                        p.id as project_id,
                        pgi.id as project_github_id,
                        pgi.last_commit as last_commit,
                        pgi.dev_commit as dev_commit,
                        pgi.branch_name as branch_name,
                        ps.git_commit as prod_commit
                    FROM projects p 
                        left join projects_github pgi on p.project_github_id = pgi.id
                        left join projects_snapshots ps on ps.project_id = p.id
                    WHERE p.id = $1",
                    proj_slug.id
                )
                    .fetch_one(&db)
                    .await?;
                sqlx::query!(
                    "UPDATE projects_github SET auto_deploy = $1 WHERE id = $2",
                    auto_deploy,
                    project_git.project_github_id,
                )
                    .execute(&db)
                    .await?;
                if auto_deploy && !project_git.prod_commit.clone().unwrap_or_default().eq(&project_git.last_commit) {
                    ssr::handle_auto_deploy_git(
                        &db,
                        ws_clients()?,
                        server_id,
                        proj_slug,
                        project_git.project_github_id,
                        project_git.branch_name,
                        project_git.dev_commit,
                        project_git.prod_commit,
                        project_git.last_commit,
                    ).await?;
                }
                Ok(())
            },
        )
            .await
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn update_default_branch_action(
        csrf: String,
        server_id: ServerId,
        project_slug: ProjectSlugStr,
        new_branch_name: String,
        new_branch_commit: String,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Write,
            Some(csrf),
            move |auth_user_id, db, proj_slug| async move {
                let project_git = sqlx::query!(
                    "SELECT 
                        pgi.id as project_github_id, 
                        pgi.auto_deploy as auto_deploy,
                        ps.git_commit as prod_commit
                    FROM projects p 
                        left join projects_github pgi on p.project_github_id = pgi.id
                        left join projects_snapshots ps on ps.project_id = p.id
                    WHERE p.id = $1",
                    proj_slug.id
                )
                    .fetch_one(&db)
                    .await?;
                let dev_commit = sqlx::query!(
                    "UPDATE projects_github SET branch_name = $1, last_commit = $2 WHERE id = $3 returning dev_commit",
                    new_branch_name,
                    new_branch_commit,
                    project_git.project_github_id,
                )
                    .fetch_one(&db)
                    .await?;
                
                if project_git.auto_deploy {
                    ssr::handle_auto_deploy_git(
                        &db,
                        ws_clients()?,
                        server_id,
                        proj_slug,
                        project_git.project_github_id,
                        new_branch_name.clone(),
                        dev_commit.dev_commit,
                        project_git.prod_commit,
                        new_branch_commit,
                    ).await?;
                }else{
                    ssr::inner_update_dev_with_git(
                        &db,
                        ws_clients()?,
                        server_id,
                        proj_slug,
                        project_git.project_github_id,
                        new_branch_name,
                        new_branch_commit,
                    ).await?;
                }
               
                Ok(())
            },
        )
            .await
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn delete_project(
        csrf: String,
        server_id: ServerId,
        project_slug: ProjectSlugStr,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner,
            Some(csrf),
            |_, db, project_slug| async move {
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
                let user_slugs = users
                    .into_iter()
                    .map(|u| Slug::new(u.id, u.username))
                    .collect::<Vec<_>>();
                let snapshot_names = sqlx::query!( 
                    "DELETE FROM projects_snapshots WHERE project_id = $1 RETURNING snapshot_name",
                    project_slug.id
                ).fetch_all(&db).await?;
                let active_id = sqlx::query!(
                    "delete from projects where id = $1 returning active_snapshot_id",
                    project_slug.id
                )
                    .fetch_one(&db)
                    .await?
                    .active_snapshot_id;
                if active_id.is_some() {
                    request_server_project_action(server_id, project_slug.clone(), ProjectSnapshotAction::UnmountProd.into(), None).await?;
                }
                for snapshot in snapshot_names {
                    request_server_project_action(server_id, project_slug.clone(), ProjectSnapshotAction::Delete { snapshot_name: snapshot.snapshot_name }.into(), None).await?;
                }


                request_user_action(
                    server_id,
                    ServerUserAction::RemoveProject {
                        user_slugs,
                        project_slug,
                    },
                )
                    .await?;
                leptos_axum::redirect("/user/projects");

                Ok(())
            },
        )
            .await
    }


    #[cfg(feature = "ssr")]
    pub mod ssr {
        use common::{ProjectSlugStr, ServerId, Slug};
        use common::server_action::project_action::git_action::ProjectGitAction;
        use crate::api::ssr::request_server_project_action;
        use crate::app::pages::user::projects::project::project_settings::commit_display;
        use crate::app::pages::user::projects::project::project_settings::server_fns::ssr;
        use crate::app::pages::user::projects::project::project_snapshots::server_fns::ssr::inner_set_snapshot_prod;
        use crate::app::pages::user::projects::project::project_snapshots::server_fns::ssr::inner_create_snapshot;
        use crate::AppResult;
        use crate::ssr::WsClients;

        pub async fn inner_update_dev_with_git(
            pool:&sqlx::PgPool,
            ws_clients: WsClients,
            server_id:ServerId,
            project_slug:Slug,
            project_github_id:i64,
            branch_name:String,
            last_commit:String,
        )->AppResult<()>{
            sqlx::query!(
                "UPDATE projects_github SET dev_commit = $1 WHERE id = $2",
                last_commit,
                project_github_id,
            )
                .execute(pool)
                .await?;

            request_server_project_action(
                server_id,
                project_slug,
                ProjectGitAction::Pull {
                    branch: branch_name,
                    commit: last_commit,
                }.into(),
                Some(ws_clients),
            ).await?;
            Ok(())
        }
        

        pub async fn handle_auto_deploy_git(
            pool: &sqlx::PgPool,
            ws_clients: WsClients,
            server_id: ServerId,
            project_slug: Slug,
            project_github_id: i64,
            branch_name:String,
            dev_commit:String,
            prod_commit:Option<String>,
            last_commit:String,
        ) -> AppResult<()> {
            let mut dev_commit = dev_commit;
            if !dev_commit.eq(&last_commit){
                inner_update_dev_with_git(
                    pool,
                    ws_clients.clone(),
                    server_id,
                    project_slug.clone(),
                    project_github_id,
                    branch_name.clone(),
                    last_commit.clone(),
                ).await?;
                dev_commit =  last_commit;
            }
            if prod_commit.is_none() || !prod_commit.clone().unwrap_or_default().eq(&dev_commit) {
                let project_snapshot_id = inner_create_snapshot(pool,ws_clients.clone(), server_id, project_slug.clone(), None, None,Some(branch_name.clone()), Some(dev_commit.clone())).await?;
                inner_set_snapshot_prod(pool,ws_clients.clone(), server_id, project_slug.clone(), project_snapshot_id).await?;
            }
            
            Ok(())
        }
    }
}
