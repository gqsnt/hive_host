use crate::app::components::select::{FormSelectIcon};
use crate::app::pages::user::projects::project::{MemoProjectParams, ProjectSlugSignal};
use crate::app::IntoView;

use common::permission::Permission;

use crate::app::components::csrf_field::CSRFField;
use leptos::either::{Either, EitherOf3};
use leptos::prelude::{signal, AddAnyAttr, Effect, Read, ServerFnError, Set, Signal};
use leptos::prelude::CollectView;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::{
    expect_context, ActionForm, ClassAttribute, For, Get, IntoMaybeErased, Resource, ServerAction,
    Show, Suspend, Suspense,
};
use leptos::{component, view};
use leptos::logging::log;
use reactive_stores::Store;
use strum::IntoEnumIterator;
use crate::app::pages::{GlobalState, GlobalStateStoreFields};

#[component]
pub fn ProjectTeam() -> impl IntoView {
    let global_state:Store<GlobalState> = expect_context();
    let project_slug_signal:Signal<ProjectSlugSignal> = expect_context();
    let slug = move ||
        project_slug_signal.read().0.clone(); 

    let update_member = ServerAction::<server_fns::UpdateProjectTeamPermission>::new();
    let add_member = ServerAction::<server_fns::AddProjectTeamPermission>::new();
    let delete_member = ServerAction::<server_fns::DeleteProjectTeamMember>::new();
    
    let team_res = Resource::new(
        move || {
            (
                update_member.version().get(),
                add_member.version().get(),
                delete_member.version().get(),
                slug()
            )
        },
        move |(u, a, d, s)| {
            log!("Fetching team for with u:{} a:{}, d:{}, s:{}", u, a, d, s);
            server_fns::get_project_team(s)
        },
    );

    view! {
        <div>
            <h2 class="section-title">"Team"</h2>
            <p class="section-description">"Manage project team members and permissions."</p>

            <Suspense fallback=move || {
                view! { <p class="text-gray-400">"Loading team..."</p> }
            }>
                {move || Suspend::new(async move {
                    match team_res.get() {
                        Some(Ok(project_response)) => {
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
                                    Some(Err(ServerFnError::ServerError(e))) => {
                                        set_add_member_result.set(e.to_string())
                                    }
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
                                                    each=move || project_response.user_permissions.clone()
                                                    key=|p| p.user_id
                                                    let(perm)
                                                >
                                                    <tr>
                                                        <td class="table-td">{perm.username.clone()}</td>
                                                        <td class="px-4 py-3">
                                                            <Show
                                                                when=move || project_response.is_owner
                                                                fallback=move || {
                                                                    view! {
                                                                        <span class="text-gray-500">
                                                                            {perm.permission.to_string()}
                                                                        </span>
                                                                    }
                                                                }
                                                            >
                                                                <ActionForm action=update_member>
                                                                    <input type="hidden" name="project_slug" value=slug() />
                                                                    <input type="hidden" name="user_id" value=perm.user_id />
                                                                    <CSRFField />
                                                                    <div class="flex flex-col gap-y-2 lg:flex-row lg:items-center lg:gap-x-4">
                                                                        <div class="relative">
                                                                            <select name="permission" class="form-select">
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
                                                                </ActionForm>
                                                            </Show>
                                                        </td>
                                                        <td class="px-4 py-3">
                                                            <Show
                                                                when=move || project_response.is_owner
                                                                fallback=move || view! {}
                                                            >
                                                                <ActionForm action=delete_member on:submit=move |_| {}>
                                                                    <input type="hidden" name="project_slug" value=slug() />
                                                                    <input type="hidden" name="user_id" value=perm.user_id />
                                                                    <CSRFField />
                                                                    <button type="submit" class="btn btn-danger">
                                                                        "Remove"
                                                                    </button>
                                                                </ActionForm>
                                                            </Show>
                                                        </td>
                                                    </tr>
                                                </For>

                                            </tbody>
                                        </table>

                                        <Show when=move || project_response.is_owner>
                                            <div class="pt-6 section-border">
                                                <h3 class="section-title">"Add Member"</h3>
                                                <ActionForm action=add_member>
                                                    <input type="hidden" name="project_slug" value=slug() />
                                                    <CSRFField />
                                                    <div class="mt-4 flex flex-col gap-y-4">
                                                        <div class="flex flex-col gap-y-2 lg:flex-row lg:gap-x-6">
                                                            <div class="flex-1">
                                                                <label for="email" class="form-label">
                                                                    "Email"
                                                                </label>
                                                                <input
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
                                                                    <select name="permission" class="form-select">
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
                                                </ActionForm>
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
                                        {format!("Error fetching team: {}", e)}
                                    </p>
                                },
                            )
                        }
                        None => {
                            EitherOf3::C(view! { <p class="text-gray-400">"Loading team..."</p> })
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}

pub mod server_fns {

    use common::permission::Permission;
    use common::{ProjectId, ProjectSlugStr, UserId};
    use leptos::prelude::ServerFnError;
    use leptos::server;
    use serde::{Deserialize, Serialize};


    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
         use crate::security::utils::ssr::get_auth_session_user_id;
        use crate::api::ssr::request_server_project_action;
        use common::server_project_action::permission::PermissionAction;
        use common::{ProjectSlug, UserSlug};
        use crate::security::permission::ssr::handle_project_permission_request;
       use validator::ValidationError;
           use std::borrow::Cow;
    }}

    #[server]
    pub async fn delete_project_team_member(
        csrf: String,
        project_slug: ProjectSlugStr,
        user_id: UserId,
    ) -> Result<(), ServerFnError> {
        Ok(handle_project_permission_request(
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
                let user_slug = UserSlug::new(user_id, other_user.username);
                let project_slug = ProjectSlug::new(project.id, project.name);
                request_server_project_action(
                    project_slug,
                    PermissionAction::Revoke { user_slug }.into(),
                )
                .await?;
                Ok(())
            },
        )
        .await?)
    }

    #[server]
    pub async fn update_project_team_permission(
        csrf: String,
        project_slug: ProjectSlugStr,
        user_id: UserId,
        permission: Permission,
    ) -> Result<(), ServerFnError> {
        Ok(handle_project_permission_request(
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
                let user_slug = UserSlug::new(user_id, other_user.username);
                let project_slug = ProjectSlug::new(project.id, project.name);
                request_server_project_action(
                    project_slug,
                    PermissionAction::Update {
                        user_slug,
                        permission,
                    }
                    .into(),
                )
                .await?;
                Ok(())
            },
        )
        .await?)
    }





    #[server]
    pub async fn add_project_team_permission(
        csrf: String,
        project_slug: ProjectSlugStr,
        email: String,
        permission: Permission,
    ) -> Result<(), ServerFnError> {
        Ok(handle_project_permission_request(
            project_slug,
            Permission::Owner,
            Some(csrf),
            |_, pool, project_slug| async move {
                let other_user =
                    sqlx::query!(r#"SELECT id,username FROM users WHERE email = $1"#, email)
                        .fetch_one(&pool)
                        .await;
                let other_user = match other_user {
                    Ok(r) => {r}
                    Err(_) =>  return Err(ValidationError::new(
                        "user_not_found",
                    ).with_message(Cow::from("User not found")).into())
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
                let user_slug = UserSlug::new(other_user.id, other_user.username);
                let project_slug = ProjectSlug::new(project.id, project.name);
                request_server_project_action(
                    project_slug,
                    PermissionAction::Grant {
                        user_slug,
                        permission,
                    }
                    .into(),
                )
                .await?;
                Ok(())
            },
        )
        .await?)
    }

    #[server]
    pub async fn get_project_team(
        project_slug: ProjectSlugStr,
    ) -> Result<ProjectTeamResponse, ServerFnError> {
        Ok(handle_project_permission_request(
            project_slug,
            Permission::Read,
            None,
            |auth, pool, project_slug| async move {
                let user_permissions = sqlx::query_as!(
                UserPermissionPage,
                r#"
                    SELECT user_id,project_id, permission as "permission: Permission", u.username as username
                    FROM permissions
                    INNER JOIN public.users u on u.id = permissions.user_id
                    WHERE project_id = $1"#,
                project_slug.id
            ).fetch_all(&pool).await?;
                let user_id = get_auth_session_user_id(&auth).unwrap();
                let is_owner = user_permissions
                    .iter()
                    .any(|p| p.user_id == user_id && p.permission == Permission::Owner);

                Ok(ProjectTeamResponse {
                    project_id:project_slug.id,
                    is_owner,
                    user_permissions,
                })
            }
        )
            .await?)
    }

    #[derive(Clone, Deserialize, Debug, Serialize, Default)]
    pub struct ProjectTeamResponse {
        pub is_owner: bool,
        pub project_id: ProjectId,
        pub user_permissions: Vec<UserPermissionPage>,
    }

    #[derive(Clone, Deserialize, Debug, Serialize)]
    #[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
    pub struct UserPermissionPage {
        pub user_id: UserId,
        pub project_id: ProjectId,
        pub username: String,
        pub permission: Permission,
    }
}
