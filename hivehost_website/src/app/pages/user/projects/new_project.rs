use crate::app::pages::user::projects::new_project::server_fns::{
    get_github_accounts, get_servers,
};
use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use crate::github::GithubRepo;
use crate::AppResult;
use leptos::either::{Either, EitherOf3};
use leptos::ev::Targeted;
use leptos::html::{Input, Select};
use leptos::logging::log;
use leptos::prelude::{
    event_target_value, expect_context, signal, Effect, ElementChild, Get, GetUntracked, NodeRef,
    NodeRefAttribute, OnAttribute, OnceResource, ReadSignal, Set, Show, StyleAttribute, Suspend,
    Suspense, Transition, WriteSignal,
};
use leptos::prelude::{ClassAttribute, IntoMaybeErased};
use leptos::prelude::{OnTargetAttribute, Resource, Update};
use leptos::server::ServerAction;
use leptos::{component, view, IntoView};
use reactive_stores::Store;
use std::sync::Arc;
use web_sys::{Event, HtmlSelectElement, MouseEvent};
use crate::github::GithubBranch;

pub type GithubInfo = (Option<i64>, String, GithubBranch);

#[component]
pub fn NewProjectPage(
    create_project_action: ServerAction<server_fns::CreateProject>,
) -> impl IntoView {
    let global_store: Store<GlobalState> = expect_context();

    let (github_info, set_github_info) = signal(None::<GithubInfo>);
    let (github_info_is_public, set_github_info_is_public) = signal(None::<bool>);
    let servers_resource = OnceResource::new_bincode(get_servers());

    let (new_project_result, set_new_project_result) = signal(" ".to_string());
    Effect::new(move |_| {
        create_project_action.version().get();
        match create_project_action.value().get() {
            Some(Ok(_)) => set_new_project_result.set(String::from("Project created")),
            Some(Err(e)) => set_new_project_result.set(e.to_string()),
            _ => (),
        };
    });
    let project_name_ref = NodeRef::<Input>::default();
    let server_id_ref = NodeRef::<Select>::default();

    let handle_select_github_type = move |ev: MouseEvent, public_type: bool| {
        ev.prevent_default();
        set_github_info_is_public(Some(public_type));
        set_github_info(None);
    };

    let handle_clear_github_selection = move |ev: MouseEvent| {
        ev.prevent_default();
        set_github_info_is_public(None);
        set_github_info(None);
    };

    let on_new_project = move |event: web_sys::SubmitEvent| {
        event.prevent_default();
        create_project_action.dispatch(server_fns::CreateProject {
            server_id: server_id_ref
                .get()
                .expect("<select> should be mounted")
                .value()
                .parse()
                .unwrap(),
            csrf: global_store.csrf().get().unwrap_or_default(),
            name: project_name_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
            github_info: github_info(),
        });
    };

    view! {
        <div class="section-border">
            <h2 class="section-title">"New Project"</h2>
            <p class="section-desc">"Create a new project."</p>

            <form on:submit=on_new_project>
                <div class="mt-10 grid grid-cols-1 gap-x-6 gap-y-8 sm:grid-cols-6">
                    <div class="sm:col-span-4">
                        <label for="name" class="form-label">
                            "Project Name"
                        </label>
                        <div class="mt-2">
                            <input
                                type="text"
                                node_ref=project_name_ref
                                name="name"
                                required
                                class="form-input"
                            />
                        </div>
                    </div>
                </div>
                <Transition fallback=|| {
                    view! { <div>"Loading data ..."</div> }
                }>
                    {move || Suspend::new(async move {
                        let servers = servers_resource.await;
                        match servers {
                            Ok(servers) => {
                                Either::Left(

                                    view! {
                                        <div class="mt-6">
                                            <label for="server" class="form-label">
                                                "Select Server"
                                            </label>
                                            <select
                                                name="server"
                                                class="form-select"
                                                node_ref=server_id_ref
                                            >
                                                {servers
                                                    .into_iter()
                                                    .map(|server| {
                                                        view! { <option value=server.id>{server.name}</option> }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            </select>
                                        </div>
                                    },
                                )
                            }
                            Err(_) => {
                                Either::Right(

                                    view! { <div>"Error loading servers"</div> },
                                )
                            }
                        }
                    })}
                </Transition>
                <h3 class="text-lg font-semibold text-white mt-12 mb-6 border-t border-white/10 pt-6">
                    "Link project to GitHub (optional)"
                </h3>

                {move || match github_info_is_public.get() {
                    None => {
                        view! {
                            <div class="p-6 bg-gray-800 rounded-lg border border-white/10 text-center space-y-4 my-4">
                                <p class="text-md text-gray-400">
                                    "Choose how to link your GitHub repository, or skip this step to create the project without a GitHub link."
                                </p>
                                <div class="flex justify-center space-x-4">
                                    <button
                                        type="button"
                                        class="btn btn-secondary"
                                        on:click=move |ev| handle_select_github_type(ev, true)
                                    >
                                        "Link Public Repository"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-secondary"
                                        on:click=move |ev| handle_select_github_type(ev, false)
                                    >
                                        "Link Private Repository"
                                    </button>
                                </div>
                            </div>
                        }
                    }
                    Some(is_public_type) => {
                        view! {
                            <div class="my-4">
                                <div class="flex justify-between items-center mb-4">
                                    <h4 class="text-md font-semibold text-white">
                                        {if is_public_type {
                                            "Configuring Public Repository"
                                        } else {
                                            "Configuring Private Repository"
                                        }}
                                    </h4>
                                    <button
                                        type="button"
                                        class="text-sm text-red-400 hover:text-red-300 underline cursor-pointer"
                                        on:click=handle_clear_github_selection
                                    >
                                        "Clear GitHub Link"
                                    </button>
                                </div>
                                {if is_public_type {
                                    Either::Left(
                                        view! {
                                            <div class="p-6 rounded-lg border border-white/10 bg-gray-800 shadow-lg">
                                                <NewProjectPublicGithub
                                                    global_store
                                                    github_info
                                                    set_github_info
                                                />
                                            </div>
                                        },
                                    )
                                } else {
                                    Either::Right(
                                        view! {
                                            <div class="p-6 rounded-lg border border-white/10 bg-gray-800 shadow-lg">
                                                <NewProjectPrivateGithub global_store set_github_info />
                                            </div>
                                        },
                                    )
                                }}
                            </div>
                        }
                    }
                }}

                // Increased mt for separation
                <div class="mt-8 flex items-center justify-end gap-x-6">
                    <button type="submit" class="btn btn-primary">
                        "Create Project"
                    </button>
                </div>
                <div>{new_project_result}</div>
            </form>

        </div>
    }
}

#[component]
pub fn NewProjectPublicGithub(
    global_store: Store<GlobalState>,
    github_info: ReadSignal<Option<GithubInfo>>,
    set_github_info: WriteSignal<Option<GithubInfo>>,
) -> impl IntoView {
    let (public_repo_full_name, set_public_repo_full_name) = signal(String::new());
    let (public_branch, set_public_branch) = signal(None::<GithubBranch>);

    let public_branches_resource = Resource::new_bincode(
        move || {
            (
                global_store.csrf().get().unwrap_or_default(),
                public_repo_full_name(),
            )
        },
        |(csrf, public_repo_full_name)| {
            server_fns::get_github_public_branches(csrf, public_repo_full_name)
        },
    );

    view! {
        <div>
            <div class="grid grid-cols-1 gap-y-6">
                <div class="sm:col-span-1">
                    <label for="public_repo_full_name" class="form-label">
                        "Public Repo Name (owner/repo)"
                    </label>
                    <div class="mt-2">
                        <input
                            type="text"
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                if !value.eq(&public_repo_full_name()) && github_info().is_some() {
                                    set_github_info(None);
                                }
                                set_public_repo_full_name(value);
                            }
                            name="public_repo_full_name"
                            placeholder="username/repository-name"
                            class="form-input"
                        />
                    </div>
                </div>
                <div class="">
                    <Show when=move || {
                        !public_repo_full_name().is_empty() && public_repo_full_name().contains('/')
                    }>
                        <label for="github_public_branch" class="form-label">
                            "Select Branch"
                        </label>
                        <Suspense fallback=|| {
                            view! {
                                <div class="mt-2 text-sm text-gray-400">"Loading branches ..."</div>
                            }
                        }>
                            {move || Suspend::new(async move {
                                let branches = public_branches_resource.await;
                                let handle_branch_change = move |ev: Targeted<Event, HtmlSelectElement>, branches:Arc<Vec<GithubBranch>>|
                                {
                                    let value = ev.target().value();
                                    if value.is_empty() {
                                        set_public_branch(None);
                                        set_github_info(None);
                                    } else {
                                        let branch = branches
                                            .iter()
                                            .find(|branch| branch.commit == value)
                                        .map(|branch| branch.clone()).unwrap();
                                        set_public_branch(Some(branch.clone()));
                                        set_github_info(
                                            Some((None, public_repo_full_name(), branch)),
                                        );
                                    }
                                };
                                match branches {
                                    Ok(inner_branches) => {
                                        let inner_branches = Arc::new(inner_branches);
                                        let inner_branches_clone = inner_branches.clone();
                                        if inner_branches.is_empty()
                                            && !public_repo_full_name().is_empty()
                                        {
                                            Either::Right(
                                                view! {
                                                    <div class="mt-2 text-sm text-yellow-400">
                                                        "No branches found or repo does not exist. Check repo name."
                                                    </div>
                                                },
                                            )
                                        } else {
                                            Either::Left(
                                                view! {
                                                    <select
                                                        name="github_public_branch"
                                                        class="form-select mt-2"
                                                        on:change:target=move |ev|handle_branch_change(ev, inner_branches_clone.clone())
                                                    >
                                                        <option value="">"Select a branch..."</option>
                                                        {inner_branches
                                                            .iter()
                                                            .map(|branch| {
                                                                view! {
                                                                    <option
                                                                        value=branch.commit.clone()
                                                                    >
                                                                        {branch.name.clone()}
                                                                    </option>
                                                                }
                                                            })
                                                            .collect::<Vec<_>>()}
                                                    </select>
                                                },
                                            )
                                        }
                                    }
                                    Err(_) => {
                                        Either::Right(
                                            view! {
                                                <div class="mt-2 text-sm text-red-400">
                                                    "Error loading branches. Ensure repo name is correct."
                                                </div>
                                            },
                                        )
                                    }
                                }
                            })}
                        </Suspense>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn NewProjectPrivateGithub(
    global_store: Store<GlobalState>,
    set_github_info: WriteSignal<Option<GithubInfo>>,
) -> impl IntoView {
    let accounts_resource = OnceResource::new_bincode(get_github_accounts());
    let (github_account_id_signal, set_github_account_id_signal) = signal(None::<i64>);
    let (github_full_name_signal, set_github_repo_id_signal) = signal(None::<String>);
    let (github_branch_id_signal, set_github_branch_id_signal) = signal(None::<GithubBranch>);
    let repositories_resource = Resource::new_bincode(
        move || {
            (
                global_store.csrf().get().unwrap_or_default(),
                github_account_id_signal(),
            )
        },
        |(csrf, account_id)| server_fns::get_github_repos(csrf, account_id),
    );
    let branches_resource = Resource::new_bincode(
        move || {
            (
                global_store.csrf().get().unwrap_or_default(),
                github_account_id_signal(),
                github_full_name_signal(),
            )
        },
        |(csrf, account_id, repo)| {
            server_fns::get_github_installation_branches(csrf, account_id, repo)
        },
    );
    let handle_account_change = move |ev: Targeted<Event, HtmlSelectElement>| {
        let value = ev.target().value();

        if value.is_empty() {
            set_github_account_id_signal(None);
            set_github_info(None);
            set_github_repo_id_signal(None);
            set_github_branch_id_signal(None);
        } else {
            let git_account_id = value.parse::<i64>().unwrap();
            if github_account_id_signal() != Some(git_account_id) {
                set_github_info(None);
                set_github_repo_id_signal(None);
                set_github_branch_id_signal(None);
            }
            set_github_account_id_signal(Some(git_account_id));
        }
    };

    let handle_repo_change = move |ev: Targeted<Event, HtmlSelectElement>| {
        let value = ev.target().value();
        if value.is_empty() {
            set_github_repo_id_signal(None);
            set_github_branch_id_signal(None);
            set_github_info(None);
        } else {
            if github_full_name_signal() != Some(value.clone()) {
                set_github_branch_id_signal(None);
                set_github_info(None);
            }
            set_github_repo_id_signal(Some(value));
        }
    };

    let handle_branch_change = move |ev: Targeted<Event, HtmlSelectElement>, branches:Arc<Vec<GithubBranch>>| {
        let value = ev.target().value();
        if value.is_empty() {
            set_github_branch_id_signal(None);
            set_github_info(None);
        } else {
            let branch = branches
                .iter()
                .find(|branch| branch.commit == value)
                .map(|branch| branch.clone())
                .unwrap();
            set_github_branch_id_signal(Some(branch.clone()));
            set_github_info(Some((
                github_account_id_signal(),
                github_full_name_signal().unwrap_or_default(),
                branch,
            )));
        }
    };

    view! {
        <Transition fallback=|| {
            view! { <div class="text-sm text-gray-400">"Loading GitHub data ..."</div> }
        }>
            {move || Suspend::new(async move {
                let accounts = accounts_resource.await;
                match accounts {
                    Ok(accounts) => {
                        if accounts.is_empty() {
                            EitherOf3::A(
                                view! {
                                    <div class="text-sm text-yellow-400 p-4 bg-yellow-900/30 rounded-md">
                                        "No GitHub accounts linked. Please link a GitHub account via your "
                                        <a
                                            // Standard HTML link to the user settings page
                                            href="/user/settings"
                                            class="font-medium text-yellow-300 hover:text-yellow-200 underline transition-colors duration-150 ease-in-out"
                                        >
                                            "profile settings"
                                        </a> " to use private repositories."
                                    </div>
                                },
                            )
                        } else {
                            EitherOf3::B(
                                view! {
                                    <div class="grid grid-cols-1 gap-y-6 sm:grid-cols-1">
                                        <div>
                                            <label for="github_account" class="form-label">
                                                "Select GitHub Account"
                                            </label>
                                            <select
                                                name="github_account"
                                                class="form-select mt-2"
                                                on:change:target=handle_account_change
                                            >
                                                <option value="">"Select account..."</option>
                                                {accounts
                                                    .into_iter()
                                                    .map(|git_account| {
                                                        view! {
                                                            <option
                                                                value=git_account.id
                                                                selected=github_account_id_signal
                                                                    .get()
                                                                    .map_or(false, |id| id == git_account.id)
                                                            >
                                                                {git_account.login.clone()}
                                                            </option>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            </select>
                                        </div>

                                        <Show when=move || github_account_id_signal().is_some()>
                                            <div>
                                                <label for="github_repo" class="form-label">
                                                    "Select Repository"
                                                </label>
                                                <Suspense fallback=|| {
                                                    view! {
                                                        <div class="mt-2 text-sm text-gray-400">
                                                            "Loading repositories ..."
                                                        </div>
                                                    }
                                                }>
                                                    {move || Suspend::new(async move {
                                                        let repositories = repositories_resource.await;
                                                        match repositories {
                                                            Ok(inner_repositories) => {
                                                                if inner_repositories.is_empty() {
                                                                    Either::Right(
                                                                        view! {
                                                                            <div class="mt-2 text-sm text-yellow-400">
                                                                                "No repositories found for this account."
                                                                            </div>
                                                                        },
                                                                    )
                                                                } else {
                                                                    Either::Left(
                                                                        view! {
                                                                            <select
                                                                                name="github_repo"
                                                                                class="form-select mt-2"
                                                                                on:change:target=handle_repo_change
                                                                            >
                                                                                <option value="">"Select repository..."</option>
                                                                                {inner_repositories
                                                                                    .into_iter()
                                                                                    .map(|repo| {
                                                                                        let repo_full_name = repo.full_name.clone();
                                                                                        view! {
                                                                                            <option
                                                                                                value=repo.full_name.clone()
                                                                                                selected=github_full_name_signal
                                                                                                    .get()
                                                                                                    .map_or(false, |name| name == repo_full_name)
                                                                                            >
                                                                                                {repo.name.clone()}
                                                                                                {if repo.private {
                                                                                                    view! {
                                                                                                        <span class="text-xs text-gray-400 ml-1">"(private)"</span>
                                                                                                    }
                                                                                                } else {
                                                                                                    view! {
                                                                                                        <span class="text-xs text-green-400 ml-1">"(public)"</span>
                                                                                                    }
                                                                                                }}
                                                                                            </option>
                                                                                        }
                                                                                    })
                                                                                    .collect::<Vec<_>>()}
                                                                            </select>
                                                                        },
                                                                    )
                                                                }
                                                            }
                                                            Err(_) => {
                                                                Either::Right(
                                                                    view! {
                                                                        <div class="mt-2 text-sm text-red-400">
                                                                            "Error loading repositories."
                                                                        </div>
                                                                    },
                                                                )
                                                            }
                                                        }
                                                    })}
                                                </Suspense>
                                            </div>
                                        </Show>

                                        <Show when=move || {
                                            github_account_id_signal().is_some()
                                                && github_full_name_signal().is_some()
                                        }>
                                            <div>
                                                <label for="github_branch" class="form-label">
                                                    "Select Branch"
                                                </label>
                                                <Suspense fallback=|| {
                                                    view! {
                                                        <div class="mt-2 text-sm text-gray-400">
                                                            "Loading branches ..."
                                                        </div>
                                                    }
                                                }>
                                                    {move || Suspend::new(async move {
                                                        let branches = branches_resource.await;
                                                        match branches {
                                                            Ok(inner_branches) => {
                                                                let inner_branches= Arc::new(inner_branches);
                                                                let inner_branches_clone = inner_branches.clone(); 
                                                                if inner_branches.is_empty() {
                                                                    Either::Right(
                                                                        view! {
                                                                            <div class="mt-2 text-sm text-yellow-400">
                                                                                "No branches found for this repository."
                                                                            </div>
                                                                        },
                                                                    )
                                                                } else {
                                                                    Either::Left(
                                                                        view! {
                                                                            <select
                                                                                name="github_branch"
                                                                                class="form-select mt-2"
                                                                                on:change:target=move |ev| handle_branch_change(ev, inner_branches_clone.clone())
                                                                            >
                                                                                <option value="">"Select branch..."</option>
                                                                                {inner_branches
                                                                                    .iter()
                                                                                    .map(|branch| {
                                                                                        let branch_name = branch.clone();
                                                                                        view! {
                                                                                            <option
                                                                                                value=branch.commit.clone()
                                                                                            >
                                                                                                {branch.name.clone()}
                                                                                            </option>
                                                                                        }
                                                                                    })
                                                                                    .collect::<Vec<_>>()}
                                                                            </select>
                                                                        },
                                                                    )
                                                                }
                                                            }
                                                            Err(_) => {
                                                                Either::Right(
                                                                    view! {
                                                                        <div class="mt-2 text-sm text-red-400">
                                                                            "Error loading branches."
                                                                        </div>
                                                                    },
                                                                )
                                                            }
                                                        }
                                                    })}
                                                </Suspense>
                                            </div>
                                        </Show>
                                    </div>
                                },
                            )
                        }
                    }
                    Err(_) => {
                        EitherOf3::C(
                            view! {
                                <div class="text-sm text-red-400">
                                    "Error loading GitHub accounts."
                                </div>
                            },
                        )
                    }
                }
            })}
        </Transition>
    }
}

pub mod server_fns {

    use crate::app::pages::user::projects::new_project::GithubInfo;
    use crate::github::{GithubBranch, GithubBranchApi, GithubRepo};
    use crate::models::{Server, UserGithub};
    use crate::AppResult;
    use common::ServerId;
    use leptos::server;
    use leptos::server_fn::codec::Bincode;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use jsonwebtoken::{Algorithm, EncodingKey, Header};
        use crate::github::ssr::get_all_repos;
        use crate::security::utils::ssr::verify_easy_hash;
        use leptos::leptos_dom::log;
        use crate::security::utils::ssr::get_auth_session_user_id;
        use crate::github::ssr::get_authenticated_git_client;
            use crate::github::ssr::get_git_client;


    }}

    #[server(input=Bincode, output=Bincode)]
    pub async fn create_project(
        csrf: String,
        server_id: ServerId,
        name: String,
        github_info: Option<GithubInfo>,
    ) -> AppResult<()> {
        let auth = crate::ssr::auth(false)?;
        let server_vars = crate::ssr::server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;
        let user_slug = crate::security::utils::ssr::get_auth_session_user_slug(&auth).unwrap();
        match ssr::create_project(server_id, user_slug, name, github_info).await {
            Ok(project) => {
                log!("Project created: {:?}", project);
                leptos_axum::redirect(format!("/user/projects/{}", project.get_slug()).as_str());
            }
            Err(e) => {
                log!("Error creating default project: {:?}", e);
            }
        }

        Ok(())
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_servers() -> AppResult<Vec<Server>> {
        let auth = crate::ssr::auth(false)?;
        let pool = crate::ssr::pool()?;
        Ok(sqlx::query_as!(Server, "SELECT id, name FROM servers")
            .fetch_all(&pool)
            .await?)
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_github_accounts() -> AppResult<Vec<UserGithub>> {
        let auth = crate::ssr::auth(false)?;
        let pool = crate::ssr::pool()?;
        let user_id = get_auth_session_user_id(&auth);
        Ok(sqlx::query_as!(
            UserGithub,
            "SELECT id, login, avatar_url, html_url, installation_id,suspended FROM user_githubs WHERE user_id = $1",
            auth.current_user.unwrap().id
        ).fetch_all(&pool).await?)
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_github_public_branches(
        csrf: String,
        public_repo_full_name: String,
    ) -> AppResult<Vec<GithubBranch>> {
        if public_repo_full_name.is_empty() {
            return Ok(vec![]);
        }

        let auth = crate::ssr::auth(false)?;
        let server_vars = crate::ssr::server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;
        let client = get_git_client();

        let branches: Vec<GithubBranchApi> = client
            .get(format!(
                "https://api.github.com/repos/{}/branches",
                public_repo_full_name
            ))
            .send()
            .await?
            .json()
            .await?;

        Ok(branches.into_iter().map(|b| b.into()).collect())
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_github_installation_branches(
        csrf: String,
        user_githubs_id: Option<i64>,
        full_name: Option<String>,
    ) -> AppResult<Vec<GithubBranch>> {
        let (user_githubs_id, repo_full_name) = match (user_githubs_id, full_name) {
            (None, _) => {
                return Ok(vec![]);
            }
            (Some(user_github_id), Some(full_name)) => (user_github_id, full_name),
            _ => {
                return Ok(vec![]);
            }
        };

        let auth = crate::ssr::auth(false)?;
        let server_vars = crate::ssr::server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;

        let pool = crate::ssr::pool()?;
        let user_id = get_auth_session_user_id(&auth).unwrap();
        let github_account = sqlx::query_as!(
            UserGithub,
            "SELECT id, login, avatar_url, html_url, installation_id,suspended FROM user_githubs WHERE id = $1 and user_id = $2",
            user_githubs_id,
            user_id,
        ).fetch_one(&pool).await?;
        let (token, client) =
            get_authenticated_git_client(&server_vars, github_account.installation_id).await?;
        let branches: Vec<GithubBranchApi> = client
            .get(format!(
                "https://api.github.com/repos/{}/branches",
                repo_full_name
            ))
            .header(reqwest::header::AUTHORIZATION, format!("token {token}"))
            .send()
            .await?
            .json()
            .await?;

        Ok(branches.into_iter().map(|b| b.into()).collect())
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_github_repos(
        csrf: String,
        user_githubs_id: Option<i64>,
    ) -> AppResult<Vec<GithubRepo>> {
        let user_githubs_id = match user_githubs_id {
            None => {
                return Ok(vec![]);
            }
            Some(user_github_id) => user_github_id,
        };
        let auth = crate::ssr::auth(false)?;
        let pool = crate::ssr::pool()?;
        let server_vars = crate::ssr::server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;
        let user_id = get_auth_session_user_id(&auth).unwrap();
        let github_account = sqlx::query_as!(
            UserGithub,
            "SELECT id, login, avatar_url, html_url, installation_id,suspended FROM user_githubs WHERE id = $1 and user_id = $2",
            user_githubs_id,
            user_id,
        ).fetch_one(&pool).await?;
        let (token, client) =
            get_authenticated_git_client(&server_vars, github_account.installation_id).await?;
        Ok(get_all_repos(client, token).await?)
    }

    #[cfg(feature = "ssr")]
    pub mod ssr {
        use crate::app::pages::user::projects::new_project::GithubInfo;
        use crate::models::Project;
        use crate::security::utils::ssr::SANITIZED_REGEX;
        use crate::AppResult;
        use common::server_action::permission::Permission;
        use common::server_action::user_action::ServerUserAction;
        use common::{ServerId, Slug};
        use validator::{Validate, ValidationError};
        use crate::github::ssr::get_authenticated_git_client;
        use crate::ssr::server_vars;

        pub fn parse_repo(repo_url: &str) -> Result<(String, String, String), ValidationError> {
            if repo_url.is_empty() {
                return Err(ValidationError::new("invalid_git_repo"));
            };
            let https_git = "https://github.com/";
            let ssh_git = "git@github.com:";

            let (before, after) = if repo_url.starts_with(https_git) {
                (https_git.to_string(), repo_url.replace(https_git, ""))
            } else if repo_url.starts_with(ssh_git) {
                (ssh_git.to_string(), repo_url.replace(ssh_git, ""))
            } else {
                return Err(ValidationError::new("invalid_git_repo"));
            };
            let (user_name, repo_name) = after
                .split_once("/")
                .ok_or_else(|| ValidationError::new("invalid_git_repo"))?;

            Ok((before, user_name.to_string(), repo_name.to_string()))
        }

        pub fn validate_git_repo(repo_url: &str) -> Result<(), ValidationError> {
            parse_repo(repo_url)?;
            Ok(())
        }

        #[derive(Debug, Clone, Validate)]
        pub struct CreateProjectForm {
            #[validate(length(min = 2, max = 30), regex(path = *SANITIZED_REGEX, message="Project must contain only letters (a-z, A-Z), number (0-9) and underscores (_)"))]
            pub name: String,
        }

        pub async fn create_project(
            server_id: ServerId,
            user_slug: Slug,
            name: String,
            github_info: Option<GithubInfo>,
        ) -> AppResult<Project> {
            use crate::api::ssr::request_user_action;
            let pool = crate::ssr::pool()?;
            let project_form = CreateProjectForm { name: name.clone() };
            project_form.validate()?;

            let (project_github_id, installation_id) = match github_info.clone() {
                Some((account_id, full_name, branch)) => {
                    let (account_id, installation_id) = match account_id {
                        None => (None, None),
                        Some(account_id) => {
                            let github_account= sqlx::query!(
                                "SELECT id,user_id,installation_id, suspended FROM user_githubs WHERE id = $1 and user_id = $2 and suspended = false",
                                account_id,
                                user_slug.id,
                            )
                                .fetch_one(&pool)
                                .await?;
                            (
                                Some(github_account.id),
                                Some(github_account.installation_id),
                            )
                        }
                    };
                    let project_github =sqlx::query!(
                        "INSERT INTO projects_github (user_githubs_id, repo_full_name, branch_name, current_commit,last_commit) VALUES ($1, $2, $3,$4,$5) returning id",
                        account_id,
                        full_name,
                        branch.name,
                        branch.commit.clone(),
                        branch.commit
                    )
                        .fetch_one(&pool)
                        .await?;
                    (Some(project_github.id), installation_id)
                }
                None => (None, None),
            };

            let project_id = sqlx::query!(
                "INSERT INTO projects (name, server_id, project_github_id) VALUES ($1, $2, $3) returning id",
                project_form.name,
                server_id,
                project_github_id
            )
            .fetch_one(&pool)
            .await?.id;
            sqlx::query!(
                "INSERT INTO permissions (user_id, project_id, permission) VALUES ($1, $2, $3)",
                user_slug.id,
                project_id,
                Permission::Owner as Permission
            )
            .execute(&pool)
            .await?;
            let github_info_server=match (github_info, installation_id){
                (Some((_, full_name, branch)), Some(installation_id)) => {
                    let server_vars = server_vars()?;
                    let (token, client) = get_authenticated_git_client(&server_vars, installation_id).await?;
                    Some((Some(token),full_name, branch.name))
                }
                (Some((_, full_name, branch)), None) => {
                    Some((None, full_name, branch.name))
                }
                _ => None
            };

            request_user_action(
                server_id,
                ServerUserAction::AddProject {
                    user_slug,
                    project_slug: Slug::new(project_id, project_form.name),
                    github_info:github_info_server
                },
            )
            .await?;
            Ok(Project {
                id: project_id,
                name,
                ..Default::default()
            })
        }
    }
}
