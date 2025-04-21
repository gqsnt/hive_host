use crate::app::components::select::FormSelect;
use crate::app::pages::user::projects::project::MemoProjectParams;
use crate::app::IntoView;

use common::permission::Permission;

use common::{ProjectId, ProjectSlugStr, UserId};
use leptos::either::Either;
use leptos::prelude::AddAnyAttr;
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::{
    expect_context, ActionForm, ClassAttribute, For, Get, IntoMaybeErased, Resource,
    ServerAction, ServerFnError, Show, Suspend, Suspense,
};
use leptos::prelude::{CollectView};
use leptos::{component, server, view};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use crate::app::components::csrf_field::CSRFField;

#[component]
pub fn ProjectTeam() -> impl IntoView {
    let params: MemoProjectParams = expect_context();
    let slug = move || params.get().unwrap().project_slug.clone();

    let update_action = ServerAction::<UpdateProjectTeamPermission>::new();
    let add_action = ServerAction::<AddProjectTeamPermission>::new();
    let delete_action = ServerAction::<DeleteProjectTeamMember>::new();

    let team_res = Resource::new(
        move || {
            (
                update_action.version().get(),
                add_action.version().get(),
                delete_action.version().get(),
                slug(),
            )
        },
        move |(_, _, _, s)| get_project_team(s),
    );
    let team_data = move || {
        team_res
            .get()
            .map(|r| r.unwrap_or_default())
            .unwrap_or_default()
    };




    view! {
        <div >
            <h2 class="section-title">"Team"</h2>
            <p class="section-description">"Manage project team members and permissions."</p>

            <Suspense fallback=move || {
                view! { <p class="text-gray-400">"Loading team..."</p> }
            }>
                {move || Suspend::new(async move {
                    let data = team_data();
                    let is_owner = move || data.is_owner;
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
                                        each=move || data.user_permissions.clone()
                                        key=|p| p.user_id
                                        children=move |perm| {
                                            view! {
                                                <tr>
                                                    <td class="table-td">{perm.username.clone()}</td>
                                                    <td class="px-4 py-3">
                                                        {move || match is_owner() {
                                                            true => {
                                                                Either::Left(
                                                                    view! {
                                                                        <ActionForm action=update_action>
                                                                            <input type="hidden" name="project_slug" value=slug() />
                                                                            <input type="hidden" name="user_id" value=perm.user_id />
                                                                            <CSRFField/>
                                                                            <div class="flex flex-col gap-y-2 lg:flex-row lg:items-center lg:gap-x-4">
                                                                                <FormSelect name="permission"
                                                                                    .to_string()>
                                                                                    {Permission::iter()
                                                                                        .map(|p| {
                                                                                            view! {
                                                                                                <option value=p.to_string() selected=perm.permission == p>
                                                                                                    {p.label()}
                                                                                                </option>
                                                                                            }
                                                                                        })
                                                                                        .collect_view()}
                                                                                </FormSelect>
                                                                                <button type="submit" class="btn-primary">
                                                                                    "Update"
                                                                                </button>
                                                                            </div>
                                                                        </ActionForm>
                                                                    },
                                                                )
                                                            }
                                                            false => {
                                                                Either::Right(

                                                                    view! {
                                                                        <span class="text-gray-500">
                                                                            {perm.permission.to_string()}
                                                                        </span>
                                                                    },
                                                                )
                                                            }
                                                        }}
                                                    </td>
                                                    <td class="px-4 py-3">
                                                        <Show when=is_owner>
                                                            <ActionForm action=delete_action on:submit=move |_| {}>
                                                                <input type="hidden" name="project_slug" value=slug() />
                                                                <input type="hidden" name="user_id" value=perm.user_id />
                                                                <CSRFField/>
                                                                <button type="submit" class="btn-danger">
                                                                    "Remove"
                                                                </button>
                                                            </ActionForm>
                                                        </Show>
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    ></For>
                                </tbody>
                            </table>

                            <Show when=is_owner>
                                <div class="pt-6 section-border">
                                    <h3 class="section-title">"Add Member"</h3>
                                    <ActionForm action=add_action>
                                        <input type="hidden" name="project_slug" value=slug() />
                                        <CSRFField/>
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
                                                    <FormSelect name="permission"
                                                        .to_string()>
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
                                                    </FormSelect>

                                                </div>
                                            </div>
                                            <button type="submit" class="btn-primary">
                                                "Add"
                                            </button>
                                        </div>
                                    </ActionForm>
                                </div>
                            </Show>
                        </div>
                    }
                })}
            </Suspense>
        </div>
    }
}

#[server]
pub async fn delete_project_team_member(
    csrf: String,
    project_slug: ProjectSlugStr,
    user_id: UserId,
) -> Result<(), ServerFnError> {
    use crate::api::ssr::request_server_project_action;
    use common::server_project_action::permission::PermissionAction;
    use common::{ProjectSlug, UserSlug};

    crate::security::permission::ssr::handle_project_permission_request(
        project_slug,
        Permission::Owner,
        Some(csrf),
        |_, pool, project_slug| async move {
            let other_user =
                sqlx::query!(r#"SELECT id,username FROM users WHERE id = $1"#, user_id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            sqlx::query!(
                r#"DELETE FROM permissions WHERE user_id = $1 AND project_id = $2"#,
                user_id,
                project_slug.id
            )
                .execute(&pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            let project =
                sqlx::query!(r#"SELECT id, name FROM projects WHERE id = $1"#, project_slug.id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            let user_slug = UserSlug::new(user_id, other_user.username);
            let project_slug = ProjectSlug::new(project.id, project.name);
            request_server_project_action(
                project_slug,
                PermissionAction::Revoke { user_slug }.into(),
            )
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(())
        }
    )
        .await
}

#[server]
pub async fn update_project_team_permission(
    csrf: String,
    project_slug: ProjectSlugStr,
    user_id: UserId,
    permission: Permission,
) -> Result<(), ServerFnError> {
    use crate::api::ssr::request_server_project_action;
    use common::server_project_action::permission::PermissionAction;
    use common::{ProjectSlug, UserSlug};
    crate::security::permission::ssr::handle_project_permission_request(
        project_slug,
        Permission::Owner,
        Some(csrf),
        |_, pool, project_slug| async move {
            let other_user =
                sqlx::query!(r#"SELECT id,username FROM users WHERE id = $1"#, user_id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            sqlx::query!(
                r#"UPDATE permissions SET permission = $1 WHERE user_id = $2 AND project_id = $3"#,
                permission as Permission,
                user_id,
                project_slug.id
            )
                .execute(&pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            let project =
                sqlx::query!(r#"SELECT id, name FROM projects WHERE id = $1"#, project_slug.id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
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
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(())

    }
    )
        .await

}
#[server]
pub async fn add_project_team_permission(
    csrf: String,
    project_slug: ProjectSlugStr,
    email: String,
    permission: Permission,
) -> Result<(), ServerFnError> {
    use common::server_project_action::permission::PermissionAction;
    use common::{ProjectSlug, UserSlug};

    crate::security::permission::ssr::handle_project_permission_request(
        project_slug,
        Permission::Owner,
        Some(csrf),
        |_, pool, project_slug| async move {
            let other_user =
                sqlx::query!(r#"SELECT id,username FROM users WHERE email = $1"#, email)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            sqlx::query!(
                r#"INSERT INTO permissions (user_id, project_id, permission) VALUES ($1, $2, $3)"#,
                other_user.id,
                project_slug.id,
                permission as Permission
            )
                .execute(&pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            let project =
                sqlx::query!(r#"SELECT id, name FROM projects WHERE id = $1"#, project_slug.id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            let user_slug = UserSlug::new(other_user.id, other_user.username);
            let project_slug = ProjectSlug::new(project.id, project.name);
            crate::security::permission::request_server_project_action_front(
                project_slug.to_str(),
                PermissionAction::Grant {
                    user_slug,
                    permission,
                }
                    .into(),
                None
            )
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(())
        }
    )
        .await
}

#[server]
pub async fn get_project_team(
    project_slug: ProjectSlugStr,
) -> Result<ProjectTeamResponse, ServerFnError> {
    use crate::security::utils::ssr::get_auth_session_user_id;
    crate::security::permission::ssr::handle_project_permission_request(
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
            ).fetch_all(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;
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
    .await
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
