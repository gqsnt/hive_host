use crate::server_action::{
    add_user_to_project, remove_user_from_project_commands, update_user_in_project,
};
use crate::{ServerError, ServerResult, TarpcHelperClient, TarpcHostingClient};

use common::command::run_external_command;
use common::helper_command::{HelperCommand, HelperResponse};
use common::hosting_command::HostingCommand;
use common::server_action::project_action::git_action::ProjectGitAction;
use common::server_action::project_action::io_action::dir_action::{
    LsElement, ProjectIoDirAction, ServerProjectIoDirActionLsResponse,
};
use common::server_action::project_action::io_action::file_action::ProjectIoFileAction;
use common::server_action::project_action::io_action::ProjectIoAction;
use common::server_action::project_action::permission::ProjectPermissionAction;
use common::server_action::project_action::snapshot::ProjectSnapshotAction;
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::{ensure_path_in_project_path, get_project_dev_path, ProjectSlugStr};
use tracing::info;

pub async fn handle_server_project_action(
    hosting_client: TarpcHostingClient,
    helper_client: TarpcHelperClient,
    project_slug: ProjectSlugStr,
    action: ProjectAction,
) -> ServerResult<ProjectResponse> {
    info!("Server Project action: {:?}", action);
    match action {
        ProjectAction::Io(io) => handle_server_project_action_io(project_slug, io).await,
        ProjectAction::Permission(permission) => {
            handle_server_project_action_permission(helper_client, project_slug, permission).await
        }
        ProjectAction::Snapshot(snapshot) => {
            handle_server_project_action_snapshot(
                hosting_client,
                helper_client,
                project_slug,
                snapshot,
            )
            .await
        }
        ProjectAction::Git(git) => handle_server_project_action_git(project_slug, git).await,
    }
}

pub async fn handle_server_project_action_git(
    project_slug: ProjectSlugStr,
    action: ProjectGitAction,
) -> ServerResult<ProjectResponse> {
    Ok(match action {
        ProjectGitAction::Pull {
            branch,
            commit,
            repo_full_name,
            token,
        } => {
            let dev_path = get_project_dev_path(&project_slug);
            let token = format!("oauth2:{}@", token.0);
            let url = format!("https://{token}github.com/{}.git", repo_full_name.0);
            run_external_command("git", &["-C", &dev_path, "fetch", &url, &branch.0]).await?;
            run_external_command("git", &["-C", &dev_path, "reset", "--hard", &commit.0]).await?;
            ProjectResponse::Ok
        }
    })
}

pub async fn handle_server_project_action_snapshot(
    hosting_client: TarpcHostingClient,
    helper_client: TarpcHelperClient,
    project_slug: ProjectSlugStr,
    action: ProjectSnapshotAction,
) -> ServerResult<ProjectResponse> {
    Ok(match action {
        ProjectSnapshotAction::Create { snapshot_name } => ProjectResponse::HelperResponses(
            helper_client
                .execute(vec![HelperCommand::CreateSnapshot {
                    project_slug,
                    snapshot_name,
                }])
                .await?,
        ),
        ProjectSnapshotAction::Delete { snapshot_name } => ProjectResponse::HelperResponses(
            helper_client
                .execute(vec![HelperCommand::DeleteSnapshot { snapshot_name }])
                .await?,
        ),

        ProjectSnapshotAction::MountSnapshotProd {
            snapshot_name,
            should_umount_first,
        } => {
            let mut helper_commands = if should_umount_first {
                vec![HelperCommand::UnmountProd {
                    project_slug: project_slug.clone(),
                }]
            } else {
                vec![]
            };
            helper_commands.push(HelperCommand::MountSnapshot {
                project_slug: project_slug.clone(),
                snapshot_name,
            });

            let helper_response = helper_client.execute(helper_commands).await?;
            if helper_response == HelperResponse::Ok {
                let hosting_response = hosting_client
                    .hosting(project_slug, HostingCommand::ServeReloadProject)
                    .await?;
                ProjectResponse::HostingResponse(hosting_response)
            } else {
                ProjectResponse::HelperResponses(helper_response)
            }
        }
        ProjectSnapshotAction::UnmountProd => {
            let helper_response = helper_client
                .execute(vec![HelperCommand::UnmountProd {
                    project_slug: project_slug.clone(),
                }])
                .await?;
            if helper_response == HelperResponse::Ok {
                let hosting_response = hosting_client
                    .hosting(project_slug, HostingCommand::StopServingProject)
                    .await?;
                ProjectResponse::HostingResponse(hosting_response)
            } else {
                ProjectResponse::HelperResponses(helper_response)
            }
        }
        ProjectSnapshotAction::Restore { snapshot_name } => {
            let helper_response = helper_client
                .execute(vec![HelperCommand::RestoreSnapshot {
                    project_slug,
                    snapshot_name,
                }])
                .await?;
            ProjectResponse::HelperResponses(helper_response)
        }
    })
}

pub async fn handle_server_project_action_permission(
    helper_client: TarpcHelperClient,
    project_slug: ProjectSlugStr,
    action: ProjectPermissionAction,
) -> ServerResult<ProjectResponse> {
    let r = match action {
        ProjectPermissionAction::Grant {
            user_slug,
            permission,
        } => add_user_to_project(helper_client, user_slug, project_slug, permission).await?,
        ProjectPermissionAction::Revoke { user_slug } => {
            helper_client
                .execute(remove_user_from_project_commands(user_slug, project_slug))
                .await?
        }
        ProjectPermissionAction::Update {
            user_slug,
            permission,
        } => update_user_in_project(helper_client, user_slug, project_slug, permission).await?,
    };
    Ok(ProjectResponse::HelperResponses(r))
}
pub async fn handle_server_project_action_io(
    project_slug: ProjectSlugStr,
    action: ProjectIoAction,
) -> ServerResult<ProjectResponse> {
    match action {
        ProjectIoAction::Dir(dir) => handle_server_project_action_dir(project_slug, dir).await,
        ProjectIoAction::File(file) => handle_server_project_action_file(project_slug, file).await,
    }
}

pub async fn handle_server_project_action_dir(
    project_slug: ProjectSlugStr,
    action: ProjectIoDirAction,
) -> ServerResult<ProjectResponse> {
    match action {
        ProjectIoDirAction::Create { path } => {
            let path = ensure_path_in_project_path(&project_slug, &path, false, false).await?;
            tokio::fs::create_dir_all(path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoDirAction::Rename { path, new_name } => {
            let path = ensure_path_in_project_path(&project_slug, &path, false, true).await?;
            let new_name =
                ensure_path_in_project_path(&project_slug, &new_name, false, false).await?;
            tokio::fs::rename(path, new_name)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoDirAction::Delete { path } => {
            let path = ensure_path_in_project_path(&project_slug, &path, false, true).await?;
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoDirAction::Ls { path } => {
            let path = ensure_path_in_project_path(&project_slug, &path, false, true).await?;
            let mut entries = tokio::fs::read_dir(path).await.map_err(ServerError::from)?;
            let mut result = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let meta = entry.metadata().await?;
                result.push(LsElement {
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: meta.is_dir(),
                })
            }
            return Ok(ProjectResponse::Ls(ServerProjectIoDirActionLsResponse {
                inner: result,
            }));
        }
    }
    Ok(ProjectResponse::Ok)
}
pub async fn handle_server_project_action_file(
    project_slug: ProjectSlugStr,
    action: ProjectIoFileAction,
) -> ServerResult<ProjectResponse> {
    match action {
        ProjectIoFileAction::Create { path } => {
            ensure_path_in_project_path(&project_slug, &path, true, false).await?;
            let _writer = tokio::fs::File::create(&path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Rename { path, new_name } => {
            let path = ensure_path_in_project_path(&project_slug, &path, true, true).await?;
            let sanitized = sanitize_filename::sanitize(&new_name);
            if sanitized.is_empty() {
                return Err(ServerError::SanityCheckFailed);
            }
            let new_path = path.parent().unwrap().join(&sanitized);
            let new_path = ensure_path_in_project_path(
                &project_slug,
                &new_path.to_string_lossy(),
                true,
                false,
            )
            .await?;
            tokio::fs::rename(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Delete { path } => {
            let path = ensure_path_in_project_path(&project_slug, &path, true, true).await?;

            tokio::fs::remove_file(path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Move { path, new_path } => {
            let path = ensure_path_in_project_path(&project_slug, &path, true, true).await?;
            let new_path =
                ensure_path_in_project_path(&project_slug, &new_path, true, false).await?;
            tokio::fs::rename(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Copy { path, new_path } => {
            let path = ensure_path_in_project_path(&project_slug, &path, true, true).await?;
            let new_path =
                ensure_path_in_project_path(&project_slug, &new_path, true, false).await?;
            tokio::fs::copy(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
    }
    Ok(ProjectResponse::Ok)
}
