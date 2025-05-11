use crate::app::components::select::FormSelectIcon;
use crate::app::pages::user::projects::project::ProjectSlugSignal;
use crate::app::IntoView;

use common::server_action::permission::Permission;

use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use leptos::either::EitherOf3;
use leptos::logging::log;
use leptos::prelude::{CollectView, NodeRef, NodeRefAttribute, OnAttribute};
use leptos::prelude::ElementChild;

use leptos::prelude::{
    expect_context, ClassAttribute, For, Get, IntoMaybeErased, Resource, ServerAction,
    Show, Suspend,
};
use leptos::prelude::{signal, Effect, Read, Set, Signal, Transition};
use leptos::{component, view};
use leptos::html::{Input, Select};
use reactive_stores::Store;
use strum::IntoEnumIterator;
use common::UserId;

#[component]
pub fn ProjectTeam() -> impl IntoView {
    let global_state: Store<GlobalState> = expect_context();
    let project_slug_signal: Signal<ProjectSlugSignal> = expect_context();
    let slug = move || project_slug_signal.read().0.clone();
    let server_id = move || global_state.project().read().as_ref().unwrap().project.server_id;
    let update_member = ServerAction::<server_fns::UpdateProjectTeamPermission>::new();
    let add_member = ServerAction::<server_fns::AddProjectTeamPermission>::new();
    let delete_member = ServerAction::<server_fns::DeleteProjectTeamMember>::new();

    let add_member_email_ref= NodeRef::<Input>::default();
    let add_member_permission_ref= NodeRef::<Select>::default();

    let permission_signal = Signal::derive(move || 
        global_state
            .project()
            .read()
            .as_ref()
            .map(|p| p.permission)
            .unwrap_or_default()
    );

    let team_res = Resource::new_bincode(
        move || {
            (
                update_member.version().get(),
                add_member.version().get(),
                delete_member.version().get(),
                slug(),
            )
        },
        move |(_, _, _, s)| server_fns::get_project_team(s),
    );

    let on_update_member = move |event: web_sys::SubmitEvent, user_id: UserId, permission: String| {
        event.prevent_default();
        update_member.dispatch(server_fns::UpdateProjectTeamPermission {
            csrf: global_state.csrf().get().unwrap_or_default(),
            project_slug: slug(),
            server_id: server_id(),
            user_id,
            permission: Permission::from(permission.as_str()),
        });
    };

    let on_delete_member = move |event: web_sys::SubmitEvent, user_id: UserId| {
        event.prevent_default();
        delete_member.dispatch(server_fns::DeleteProjectTeamMember {
            csrf: global_state.csrf().get().unwrap_or_default(),
            server_id: server_id(),
            project_slug: slug(),
            user_id,
        });
    };

    let on_add_member = move |event: web_sys::SubmitEvent| {
        event.prevent_default();
        add_member.dispatch(server_fns::AddProjectTeamPermission {
            csrf: global_state.csrf().get().unwrap_or_default(),
            project_slug: slug(),
            server_id: server_id(),
            email: add_member_email_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
            permission: Permission::from(add_member_permission_ref.get().unwrap().value().as_str()),
        });
    };

    view! {
        <div>
            <h2 class="section-title">"Team"</h2>
            <p class="section-description">"Manage project team members and permissions."</p>

            <Transition fallback=move || {
                view! { <p class="text-gray-400">"Loading team..."</p> }
            }>
                {move || Suspend::new(async move {
                    match team_res.get() {
                        Some(Ok(user_permissions)) => {
                            let (add_member_result, set_add_member_result) = signal(
                                " ".to_string(),
                            );
                            Effect::new(move |_| {
                                add_member.version().get();
                                log!("add_member: {:?}", add_member.value().get());
                                match add_member.value().get() {
                                    Some(Ok(_)) => {
                                        set_add_member_result.set(String::from("Member added"))
                                    }
                                    Some(Err(e)) => set_add_member_result.set(e.to_string()),
                                    _ => {}
                                };
                            });
                            EitherOf3::A(

                                view! {
                                    <div class="mt-6 flex flex-col gap-y-8">
                                        <table class="table">
                                            <thead>
                                                <tr>
                                                    <th class="table-th">"Username"</th>
                                                    <th class="table-th">"Permission"</th>
                                                    <th class="table-th">"Actions"</th>
                                                </tr>
                                            </thead>
                                            <tbody class="divide-y divide-gray-800">
                                                <For
                                                    each=move || user_permissions.clone()
                                                    key=|p| p.user_id
                                                    let(perm)
                                                >
                                                    {
                                                        let permission_ref = NodeRef::<Select>::default();
                                                        view! {
                                                            <tr>
                                                                <td class="table-td">{perm.slug.clone()}</td>
                                                                <td class="px-4 py-3">
                                                                    <Show
                                                                        when=move || permission_signal().is_owner()
                                                                        fallback=move || {
                                                                            view! {
                                                                                <span class="text-gray-500">
                                                                                    {perm.permission.to_string()}
                                                                                </span>
                                                                            }
                                                                        }
                                                                    >
                                                                        <form on:submit=move |e| on_update_member(
                                                                            e,
                                                                            perm.user_id,
                                                                            permission_ref.get().unwrap().value(),
                                                                        )>
                                                                            <div class="flex flex-col gap-y-2 lg:flex-row lg:items-center lg:gap-x-4">
                                                                                <div class="relative">
                                                                                    <select
                                                                                        name="permission"
                                                                                        class="form-select"
                                                                                        node_ref=permission_ref
                                                                                    >
                                                                                        {Permission::iter()
                                                                                            .map(|p| {
                                                                                                view! {
                                                                                                    <option value=p.to_string() selected=perm.permission == p>
                                                                                                        {p.label()}
                                                                                                    </option>
                                                                                                }
                                                                                            })
                                                                                            .collect_view()}
                                                                                    </select>
                                                                                    <FormSelectIcon />
                                                                                </div>

                                                                                <button type="submit" class="btn btn-primary">
                                                                                    "Update"
                                                                                </button>
                                                                            </div>
                                                                        </form>
                                                                    </Show>
                                                                </td>
                                                                <td class="px-4 py-3">
                                                                    <Show when=move || permission_signal().is_owner()>
                                                                        <form on:submit=move |e| on_delete_member(e, perm.user_id)>
                                                                            <button type="submit" class="btn btn-danger">
                                                                                "Remove"
                                                                            </button>
                                                                        </form>
                                                                    </Show>
                                                                </td>
                                                            </tr>
                                                        }
                                                    }

                                                </For>

                                            </tbody>
                                        </table>

                                        <Show when=move || permission_signal().is_owner()>
                                            <div class="pt-6 section-border">
                                                <h3 class="section-title">"Add Member"</h3>
                                                <form on:submit=on_add_member>
                                                    <div class="mt-4 flex flex-col gap-y-4">
                                                        <div class="flex flex-col gap-y-2 lg:flex-row lg:gap-x-6">
                                                            <div class="flex-1">
                                                                <label for="email" class="form-label">
                                                                    "Email"
                                                                </label>
                                                                <input
                                                                    node_ref=add_member_email_ref
                                                                    type="email"
                                                                    name="email"
                                                                    required
                                                                    class="form-input"
                                                                />
                                                            </div>
                                                            <div class="flex-1">
                                                                <label for="permission" class="form-label">
                                                                    "Permission"
                                                                </label>
                                                                <div class="relative">
                                                                    <select
                                                                        name="permission"
                                                                        class="form-select"
                                                                        node_ref=add_member_permission_ref
                                                                    >
                                                                        {Permission::iter()
                                                                            .map(|p| {
                                                                                view! {
                                                                                    <option
                                                                                        value=p.to_string()
                                                                                        selected=Permission::default() == p
                                                                                    >
                                                                                        {p.label()}
                                                                                    </option>
                                                                                }
                                                                            })
                                                                            .collect_view()}
                                                                    </select>
                                                                    <FormSelectIcon />

                                                                </div>

                                                            </div>
                                                        </div>
                                                        <button type="submit" class="btn btn-primary">
                                                            "Add"
                                                        </button>
                                                    </div>
                                                    <div>{add_member_result}</div>
                                                </form>
                                            </div>
                                        </Show>
                                    </div>
                                },
                            )
                        }
                        Some(Err(e)) => {
                            EitherOf3::B(
                                view! {
                                    <p class="text-red-500">
                                        {format!("Error fetching team: {e}")}
                                    </p>
                                },
                            )
                        }
                        None => {
                            EitherOf3::C(view! { <p class="text-gray-400">"Loading team..."</p> })
                        }
                    }
                })}
            </Transition>
        </div>
    }
}

pub mod server_fns {
    use crate::AppResult;
    use common::server_action::permission::Permission;
    use common::{ProjectId, ProjectSlugStr, ServerId, UserId, UserSlugStr};
    use leptos::server;
    use leptos::server_fn::codec::Bincode;
    use serde::{Deserialize, Serialize};

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::api::ssr::request_server_project_action;
        use common::server_action::project_action::permission::ProjectPermissionAction;
        use crate::security::permission::ssr::handle_project_permission_request;
       use validator::ValidationError;
           use std::borrow::Cow;
        use common::Slug;
        use crate::ssr::permissions;
    }}

    #[server(input=Bincode, output=Bincode)]
    pub async fn delete_project_team_member(
        csrf: String,
        server_id:ServerId,
        project_slug: ProjectSlugStr,
        user_id: UserId,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner,
            Some(csrf),
            |_, pool, project_slug| async move {
                let other_user =
                    sqlx::query!(r#"SELECT id,username FROM users WHERE id = $1"#, user_id)
                        .fetch_one(&pool)
                        .await?;
                sqlx::query!(
                    r#"DELETE FROM permissions WHERE user_id = $1 AND project_id = $2"#,
                    user_id,
                    project_slug.id
                )
                .execute(&pool)
                .await?;
                let project = sqlx::query!(
                    r#"SELECT id, name FROM projects WHERE id = $1"#,
                    project_slug.id
                )
                .fetch_one(&pool)
                .await?;
                let user_slug = Slug::new(user_id, other_user.username);
                let project_slug = Slug::new(project.id, project.name);
                request_server_project_action(
                    server_id,
                    project_slug,
                    ProjectPermissionAction::Revoke { user_slug }.into(),
                )
                .await?;
                Ok(())
            },
        )
        .await
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn update_project_team_permission(
        csrf: String,
        server_id:ServerId,
        project_slug: ProjectSlugStr,
        user_id: UserId,
        permission: Permission,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner,
            Some(csrf),
            |_, pool, project_slug| async move {
                let other_user =
                    sqlx::query!(r#"SELECT id,username FROM users WHERE id = $1"#, user_id)
                        .fetch_one(&pool)
                        .await?;
                sqlx::query!(
                r#"UPDATE permissions SET permission = $1 WHERE user_id = $2 AND project_id = $3"#,
                permission as Permission,
                user_id,
                project_slug.id
            )
                    .execute(&pool)
                    .await?;
                let project = sqlx::query!(
                    r#"SELECT id, name FROM projects WHERE id = $1"#,
                    project_slug.id
                )
                .fetch_one(&pool)
                .await?;
                let user_slug = Slug::new(user_id, other_user.username);
                let project_slug = Slug::new(project.id, project.name);
                request_server_project_action(
                    server_id,
                    project_slug,
                    ProjectPermissionAction::Update {
                        user_slug,
                        permission,
                    }
                    .into(),
                )
                .await?;
                Ok(())
            },
        )
        .await
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn add_project_team_permission(
        csrf: String,
        server_id:ServerId,
        project_slug: ProjectSlugStr,
        email: String,
        permission: Permission,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner,
            Some(csrf),
            |_, pool, project_slug| async move {
                let other_user =
                    sqlx::query!(r#"SELECT id,username FROM users WHERE email = $1"#, email)
                        .fetch_one(&pool)
                        .await;
                let other_user = match other_user {
                    Ok(r) => r,
                    Err(_) => {
                        return Err(ValidationError::new("user_not_found")
                            .with_message(Cow::from("User not found"))
                            .into())
                    }
                };
                sqlx::query!(
                r#"INSERT INTO permissions (user_id, project_id, permission) VALUES ($1, $2, $3)"#,
                other_user.id,
                project_slug.id,
                permission as Permission
            )
                    .execute(&pool)
                    .await?;
                let project = sqlx::query!(
                    r#"SELECT id, name FROM projects WHERE id = $1"#,
                    project_slug.id
                )
                .fetch_one(&pool)
                .await?;
                let cache_permissions = permissions()?;
                let _ = cache_permissions
                    .remove(&(other_user.id, project_slug.id))
                    .await;
                let user_slug = Slug::new(other_user.id, other_user.username);
                let project_slug = Slug::new(project.id, project.name);
                request_server_project_action(
                    server_id,
                    project_slug,
                    ProjectPermissionAction::Grant {
                        user_slug,
                        permission,
                    }
                    .into(),
                )
                .await?;
                Ok(())
            },
        )
        .await
    }

    #[server(input=Bincode, output=Bincode)]
    pub async fn get_project_team(
        project_slug: ProjectSlugStr,
    ) -> AppResult<Vec<UserPermissionPage>> {
        handle_project_permission_request(
            project_slug,
            Permission::Read,
            None,
            |_, pool, project_slug| async move {
                let user_permissions = sqlx::query_as!(
                UserPermissionPage,
                r#"
                    SELECT user_id,project_id, permission as "permission: Permission", u.username as username, u.slug as slug 
                    FROM permissions
                    INNER JOIN public.users u on u.id = permissions.user_id
                    WHERE project_id = $1"#,
                project_slug.id
            ).fetch_all(&pool).await?;

                Ok(user_permissions)
            }
        )
            .await
    }

    #[derive(Clone, Serialize, Debug, Deserialize, Default)]
    pub struct ProjectTeamResponse {
        pub project_id: ProjectId,
        pub user_permissions: Vec<UserPermissionPage>,
    }

    #[derive(Clone, Serialize, Debug, Deserialize)]
    #[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
    pub struct UserPermissionPage {
        pub user_id: UserId,
        pub project_id: ProjectId,
        pub username: String,
        pub permission: Permission,
        pub slug:UserSlugStr,
    }
}
