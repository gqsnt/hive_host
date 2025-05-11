use crate::server_action::{
    add_user_to_project, remove_user_from_project_commands, update_user_in_project,
};
use crate::{ServerError, ServerResult, TarpcHelperClient, TarpcHostingClient};

use common::helper_command::{HelperCommand, HelperResponse};
use common::hosting_command::HostingCommand;
use common::server_action::project_action::io_action::dir_action::{
    LsElement, ProjectIoDirAction, ServerProjectIoDirActionLsResponse,
};
use common::server_action::project_action::io_action::file_action::{
    ProjectIoFileAction,
};
use common::server_action::project_action::io_action::ProjectIoAction;
use common::server_action::project_action::permission::ProjectPermissionAction;
use common::server_action::project_action::snapshot::ProjectSnapshotAction;
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::{get_project_dev_path, get_project_prod_path, get_project_snapshot_path, get_user_project_path, ProjectSlugStr};
use std::path::PathBuf;
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
    }
}
// helper_client.execute(|c,cx|async move {}).await;
pub async fn handle_server_project_action_snapshot(
    hosting_client: TarpcHostingClient,
    helper_client: TarpcHelperClient,
    project_slug: ProjectSlugStr,
    action: ProjectSnapshotAction,
) -> ServerResult<ProjectResponse> {
    Ok(match action {
        ProjectSnapshotAction::Create { snapshot_name } => {
            ProjectResponse::HelperResponses(
                helper_client
                    .execute(vec![HelperCommand::CreateSnapshot {
                        snapshot_path: get_project_snapshot_path(&snapshot_name),
                        path: get_project_dev_path(&project_slug),
                    }])
                    .await?,
            )
        },
        ProjectSnapshotAction::Delete { snapshot_name } => ProjectResponse::HelperResponses(
            helper_client
                .execute( vec![HelperCommand::DeleteSnapshot {
                    snapshot_path: get_project_snapshot_path(&snapshot_name),
                }])
                .await?,
        ),

        ProjectSnapshotAction::MountSnapshotProd {
            snapshot_name,
            should_umount_first,
        } => {
            let project_prod_path = get_project_prod_path(&project_slug);
            let mut helper_commands = if should_umount_first {
                vec![HelperCommand::UnmountProd {
                    path: project_prod_path.clone(),
                }]
            } else {
                vec![]
            };
            helper_commands.push(HelperCommand::MountSnapshot {
                path: project_prod_path,
                snapshot_name,
            });

            let helper_response = helper_client
                .execute(helper_commands)
                .await?;
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
            let project_prod_path = get_project_prod_path(&project_slug);
            let helper_response = helper_client
                .execute(vec![HelperCommand::UnmountProd {
                    path: project_prod_path,
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
        ProjectSnapshotAction::Restore { snapshot_name, users_slug } => {
            let users_project_path = users_slug
                .into_iter()
                .map(|s| get_user_project_path(&s, &project_slug))
                .collect::<Vec<String>>();
            let helper_response = helper_client
                .execute(vec![HelperCommand::RestoreSnapshot {
                    snapshot_path: get_project_snapshot_path(&snapshot_name),
                    path: get_project_dev_path(&project_slug),
                    users_project_path
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
        } => {
            add_user_to_project(
                helper_client,
                user_slug.to_string(),
                project_slug,
                permission,
            )
            .await?
        }
        ProjectPermissionAction::Revoke { user_slug } => {
            helper_client
                .execute( remove_user_from_project_commands(user_slug.to_string(), project_slug))
                .await?
        }
        ProjectPermissionAction::Update {
            user_slug,
            permission,
        } => {
            update_user_in_project(
                helper_client,
                user_slug.to_string(),
                project_slug,
                permission,
            )
            .await?
        }
    };
    Ok(ProjectResponse::HelperResponses(r))
}
pub async fn handle_server_project_action_io(
    project_slug: ProjectSlugStr,
    action: ProjectIoAction,
) -> ServerResult<ProjectResponse> {
    match action {
        ProjectIoAction::Dir(dir) => handle_server_project_action_dir(project_slug, dir).await,
        ProjectIoAction::File(file) => handle_server_project_action_file(project_slug, file).await
        
    }
}

pub async fn handle_server_project_action_dir(
    project_slug: ProjectSlugStr,
    action: ProjectIoDirAction,
) -> ServerResult<ProjectResponse> {
    match action {
        ProjectIoDirAction::Create { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, false).await?;
            tokio::fs::create_dir_all(path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoDirAction::Rename { path, new_name } => {
            let path =
                ensure_path_in_project_path(project_slug.clone(), &path, false, true).await?;
            let new_name =
                ensure_path_in_project_path(project_slug.clone(), &new_name, false, false).await?;
            tokio::fs::rename(path, new_name)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoDirAction::Delete { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, true).await?;
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoDirAction::Ls { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, true).await?;
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
            let path =
                ensure_path_in_project_path(project_slug.clone(), &path, true, false).await?;
            let _writer = tokio::fs::File::create(&path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Rename { path, new_name } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let new_path = path.parent().unwrap().join(new_name);
            tokio::fs::rename(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Delete { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, true, true).await?;
            tokio::fs::remove_file(path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Move { path, new_path } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let new_path =
                ensure_path_in_project_path(project_slug.clone(), &new_path, true, false).await?;
            tokio::fs::rename(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
        ProjectIoFileAction::Copy { path, new_path } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let new_path =
                ensure_path_in_project_path(project_slug.clone(), &new_path, true, false).await?;
            tokio::fs::copy(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
    }
    Ok(ProjectResponse::Ok)
}

pub async fn ensure_path_in_project_path(
    project_slug: ProjectSlugStr,
    project_path_: &str,
    is_file: bool,
    should_exist: bool,
) -> ServerResult<PathBuf> {
    // 1) Canonicaliser la racine projet
    let mut project_path_ = project_path_.to_string();
    if !project_path_.starts_with("root/") {
        return Err(ServerError::InvalidPath);
    }
    project_path_ = project_path_.replacen("root/", "./", 1);

    let project_root = PathBuf::from(get_project_dev_path(&project_slug.to_string()));
    let project_root = tokio::fs::canonicalize(&project_root).await?;

    // 2) Rejeter tout chemin absolu ou contenant `..`
    let rel = PathBuf::from(project_path_);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ServerError::InvalidPath);
    }

    // Chemin final (peut ne pas exister)
    let full_path = project_root.join(rel);

    if should_exist {
        // 3A) On attend que la cible existe → canonicaliser puis métadonnées
        let canon = tokio::fs::canonicalize(&full_path).await?;

        // 4A) Vérifier qu’elle reste sous project_root
        if !canon.starts_with(&project_root) {
            return Err(ServerError::OutOfProjectsScope);
        }

        // 5A) Vérifier fichier vs dossier
        let meta = tokio::fs::metadata(&canon).await?;
        if is_file && !meta.is_file() {
            return Err(ServerError::PathIsNotFile);
        }
        if !is_file && !meta.is_dir() {
            return Err(ServerError::PathIsNotDir);
        }

        Ok(canon)
    } else {
        // 3B) Création de la cible → vérifier uniquement le parent
        let parent = full_path.parent().ok_or(ServerError::PathHasNoParent)?;
        let parent_canon = tokio::fs::canonicalize(parent).await?;

        // 4B) S’assurer que le parent est dans le projet
        if !parent_canon.starts_with(&project_root) {
            return Err(ServerError::OutOfProjectsScope);
        }

        // 5B) OK pour créer : retourner le chemin (non-canon) où l’on créera.
        Ok(full_path)
    }
}
