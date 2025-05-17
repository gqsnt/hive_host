use crate::{ServerResult, TarpcHelperClient};
use common::command::run_external_command;
use common::helper_command::{HelperCommand, HelperResponse};
use common::server_action::permission::Permission;
use common::server_action::user_action::{ServerUserAction, ServerUserResponse};
use common::{
    get_project_dev_path, get_user_projects_path, GitBranchNameStr, GitRepoFullNameStr,
    GitTokenStr, ProjectSlugStr, Slug, UserSlugStr,
};
use std::path::Path;
use std::str::FromStr;
use tracing::info;

pub async fn handle_user_action(
    server_helper: TarpcHelperClient,
    action: ServerUserAction,
) -> ServerResult<ServerUserResponse> {
    info!("Server action: {:?}", action);
    let r = match action {
        ServerUserAction::Create { user_slug } => create_user(server_helper, user_slug).await?,
        ServerUserAction::Delete { user_slug } => remove_user(server_helper, user_slug).await?,
        ServerUserAction::AddProject {
            user_slug,
            project_slug,
            github_info,
        } => create_project(server_helper, user_slug, project_slug, github_info).await?,
        ServerUserAction::RemoveProject {
            user_slugs,
            project_slug,
        } => {
            let mut helper_commands = user_slugs
                .iter()
                .flat_map(|user_slug| {
                    remove_user_from_project_commands(user_slug.clone(), project_slug.clone())
                })
                .collect::<Vec<HelperCommand>>();
            helper_commands.push(HelperCommand::DeleteProject { project_slug });
            server_helper.execute(helper_commands).await?
        }
    };
    Ok(ServerUserResponse::Helper(r))
}

pub async fn create_user(
    server_helper: TarpcHelperClient,
    user_slug: UserSlugStr,
) -> ServerResult<HelperResponse> {
    Ok(server_helper
        .execute(vec![HelperCommand::CreateUser { user_slug }])
        .await?)
}

pub async fn create_project(
    server_helper: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    github_info: Option<(Option<GitTokenStr>, GitRepoFullNameStr, GitBranchNameStr)>,
) -> ServerResult<HelperResponse> {
    let dev_path = get_project_dev_path(&project_slug);
    server_helper
        .execute(vec![
            HelperCommand::CreateProject {
                project_slug: project_slug.clone(),
                user_slug: user_slug.clone(),
                with_index_html: github_info.is_none(),
            },
            HelperCommand::BindMountUserProject {
                project_slug,
                user_slug,
            },
        ])
        .await?;
    if let Some((token, full_name, branch)) = github_info {
        let token = token
            .map(|token| format!("oauth2:{}@", token.0))
            .unwrap_or_default();
        let url = format!("https://{token}github.com/{}.git", full_name.0);
        let r1 = run_external_command(
            "git",
            &[
                "clone", &url, "--branch", &branch.0, "--depth", "1", &dev_path,
            ],
        )
        .await?;
        info!("Server successfully cloned github repo: {r1}");
    }
    Ok(HelperResponse::Ok)
}

pub async fn add_user_to_project(
    server_helper: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<HelperResponse> {
    Ok(server_helper
        .execute(vec![
            HelperCommand::SetAcl {
                project_slug: project_slug.clone(),
                user_slug: user_slug.clone(),
                is_read_only: permission.is_read_only(),
            },
            HelperCommand::BindMountUserProject {
                project_slug,
                user_slug,
            },
        ])
        .await?)
}

pub async fn update_user_in_project(
    server_helper: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<HelperResponse> {
    Ok(server_helper
        .execute(vec![HelperCommand::SetAcl {
            project_slug,
            user_slug,
            is_read_only: permission.is_read_only(),
        }])
        .await?)
}

pub fn remove_user_from_project_commands(
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) -> Vec<HelperCommand> {
    vec![
        HelperCommand::RemoveAcl {
            project_slug: project_slug.clone(),
            user_slug: user_slug.clone(),
        },
        HelperCommand::UnmountUserProject {
            project_slug,
            user_slug,
        },
    ]
}

pub async fn remove_user(
    server_helper: TarpcHelperClient,
    user_slug: UserSlugStr,
) -> ServerResult<HelperResponse> {
    let user_projects_path = get_user_projects_path(&user_slug);
    let path = Path::new(&user_projects_path);
    let mut commands = vec![];
    if path.exists() {
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let submount = entry.path();
            if submount.is_dir() {
                let project_slug =
                    Slug::from_str(submount.file_name().unwrap().to_string_lossy().as_ref())?
                        .to_project_slug_str();
                commands.push(HelperCommand::UnmountUserProject {
                    project_slug,
                    user_slug: user_slug.clone(),
                });
            }
        }
    }
    commands.push(HelperCommand::DeleteUser { user_slug });
    Ok(server_helper.execute(commands).await?)
}
