use crate::cmd::project_path;
use crate::server_action::{add_user_to_project, remove_user_from_project, update_user_in_project};
use crate::{ensure_authorization, AppState, ServerError};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use common::server_project_action::io_action::dir_action::{
    DirAction, DirActionLsResponse, LsElement,
};
use common::server_project_action::io_action::file_action::{FileAction, FileInfo};
use common::server_project_action::io_action::IoAction;
use common::server_project_action::permission::PermissionAction;
use common::server_project_action::{
    IsProjectServerAction, ServerProjectAction, ServerProjectActionRequest,
    ServerProjectActionResponse,
};
use common::{ProjectUnixSlugStr, StringContent};
use secrecy::ExposeSecret;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::info;

pub async fn server_project_action_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(content): Json<StringContent>,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    if let Some((project_slug, action)) = state.server_project_action_cache.get(&token).await {
        state.server_project_action_cache.invalidate(&token).await;
        handle_server_project_action(state, project_slug, action, content).await
    } else {
        Err(ServerError::Unauthorized.into())
    }
}

pub async fn server_project_action(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ServerProjectActionRequest>,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    ensure_authorization!(headers, state, {
        if request.action.with_token() {
            if let Some(token) = request.token {
                state
                    .server_project_action_cache
                    .insert(token, (request.project_slug.to_unix(), request.action))
                    .await;
            }
            Ok(Json(ServerProjectActionResponse::Ok))
        } else {
            handle_server_project_action(
                state,
                request.project_slug.to_unix(),
                request.action,
                StringContent::default(),
            )
            .await
        }
    })
}

pub async fn handle_server_project_action(
    _state: AppState,
    project_slug: ProjectUnixSlugStr,
    action: ServerProjectAction,
    content: StringContent,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    info!("Server Project action: {:?}", action);
    match action {
        ServerProjectAction::Io(io) => {
            handle_server_project_action_io(project_slug, io, content).await
        }
        ServerProjectAction::Permission(permission) => {
            handle_server_project_action_permission(project_slug, permission).await
        }
    }
}

pub async fn handle_server_project_action_permission(
    project_slug: ProjectUnixSlugStr,
    action: PermissionAction,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        PermissionAction::Grant {
            user_slug,
            permission,
        } => {
            add_user_to_project(user_slug.to_unix(), project_slug, permission)
                .await?;
        }
        PermissionAction::Revoke { user_slug } => {
            remove_user_from_project(user_slug.to_unix(), project_slug)
                .await?;
        }
        PermissionAction::Update {
            user_slug,
            permission,
        } => {
            update_user_in_project(user_slug.to_unix(), project_slug, permission)
                .await?;
        }
    }
    Ok(Json(ServerProjectActionResponse::Ok))
}
pub async fn handle_server_project_action_io(
    project_slug: ProjectUnixSlugStr,
    action: IoAction,
    content: StringContent,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        IoAction::Dir(dir) => handle_server_project_action_dir(project_slug, dir).await,
        IoAction::File(file) => {
            handle_server_project_action_file(project_slug, file, content).await
        }
    }
}

pub async fn handle_server_project_action_dir(
    project_slug: ProjectUnixSlugStr,
    action: DirAction,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        DirAction::Create { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, false).await?;
            tokio::fs::create_dir_all(path)
                .await
                .map_err(ServerError::from)?;
        }
        DirAction::Rename { path, new_name } => {
            let path =
                ensure_path_in_project_path(project_slug.clone(), &path, false, true).await?;
            let new_name =
                ensure_path_in_project_path(project_slug.clone(), &new_name, false, false).await?;
            tokio::fs::rename(path, new_name)
                .await
                .map_err(ServerError::from)?;
        }
        DirAction::Delete { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, true).await?;
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(ServerError::from)?;
        }
        DirAction::Ls { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, true).await?;
            let mut entries = tokio::fs::read_dir(path).await.map_err(ServerError::from)?;
            let mut result = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let meta = entry.metadata().await.unwrap();
                result.push(LsElement {
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: meta.is_dir(),
                })
            }
            return Ok(Json(ServerProjectActionResponse::Ls(DirActionLsResponse {
                inner: result,
            })));
        }
        DirAction::Download => {
            // make a tar.gz of the project and send it to the client
            let project_path = project_path(project_slug.clone());
            let tar_path = format!("/tmp/{}.tar.gz", project_slug.clone());
            let tar_cmd = format!("tar -czf {} -C {} .", tar_path, project_path);
            let output = tokio::process::Command::new("bash")
                .arg("-c")
                .arg(tar_cmd)
                .output()
                .await
                .map_err(ServerError::from)?;
            if !output.status.success() {
                return Err(ServerError::CommandFailed("tar".to_string()).into());
            }
            let tar_file = tokio::fs::File::open(tar_path)
                .await
                .map_err(ServerError::from)?;
            let mut reader = tokio::io::BufReader::new(tar_file);
            // return the file to the client
            let mut buf = Vec::new();
            tokio::io::copy(&mut reader, &mut buf)
                .await
                .map_err(ServerError::from)?;
            return Ok(Json(ServerProjectActionResponse::Content(
                String::from_utf8(buf).unwrap(),
            )));
        }
    }
    Ok(Json(ServerProjectActionResponse::Ok))
}
pub async fn handle_server_project_action_file(
    project_slug: ProjectUnixSlugStr,
    action: FileAction,
    content: StringContent,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        FileAction::Create { path } => {
            let path =
                ensure_path_in_project_path(project_slug.clone(), &path, true, false).await?;
            let writer = tokio::fs::File::create(&path)
                .await
                .map_err(ServerError::from)?;
            let mut writer = tokio::io::BufWriter::new(writer);
            if let Some(content) = content.inner {
                writer
                    .write_all(content.as_bytes())
                    .await
                    .map_err(ServerError::from)?;
            }
        }
        FileAction::Rename { path, new_name } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let new_name =
                ensure_path_in_project_path(project_slug.clone(), &new_name, true, false).await?;
            tokio::fs::rename(path, new_name)
                .await
                .map_err(ServerError::from)?;
        }
        FileAction::Delete { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, true, true).await?;
            tokio::fs::remove_file(path)
                .await
                .map_err(ServerError::from)?;
        }
        FileAction::Move { path, new_path } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let new_path =
                ensure_path_in_project_path(project_slug.clone(), &new_path, true, false).await?;
            tokio::fs::rename(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
        FileAction::Copy { path, new_path } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let new_path =
                ensure_path_in_project_path(project_slug.clone(), &new_path, true, false).await?;
            tokio::fs::copy(path, new_path)
                .await
                .map_err(ServerError::from)?;
        }
        FileAction::View { path } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let path_copy = path.clone();
            let path_copy = path_copy
                .strip_prefix(project_path(project_slug.clone()))
                .map_err(ServerError::from)?;
            let name = path
                .file_name()
                .ok_or(ServerError::CantReadFileName(
                    path.to_string_lossy().to_string(),
                ))?
                .to_string_lossy()
                .to_string();
            let file = tokio::fs::File::open(path)
                .await
                .map_err(ServerError::from)?;
            let metadata = file.metadata().await.map_err(ServerError::from)?;
            let size = metadata.len();
            let modified = metadata.modified().unwrap();
            let modified: DateTime<Utc> = modified.into();
            let last_modified = modified.format("%a, %d %b %Y %T").to_string();
            let mut reader = tokio::io::BufReader::new(file);
            let mut buf = Vec::new();
            tokio::io::copy(&mut reader, &mut buf)
                .await
                .map_err(ServerError::from)?;

            return Ok(Json(ServerProjectActionResponse::File(FileInfo {
                name,
                content: String::from_utf8(buf).unwrap(),
                size,
                path: format!("./{}", path_copy.to_string_lossy()),
                last_modified,
            })));
        }
        FileAction::Update { path } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await?;
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .await
                .map_err(ServerError::from)?;
            let content = content.inner.unwrap_or_default();
            file.write_all(content.as_bytes())
                .await
                .map_err(ServerError::from)?;
        }
    }
    Ok(Json(ServerProjectActionResponse::Ok))
}

pub async fn ensure_path_in_project_path(
    project_slug: ProjectUnixSlugStr,
    user_path: &str,
    is_file: bool,
    should_exist: bool,
) -> Result<PathBuf, (StatusCode, String)> {
    // 1) Canonicaliser la racine projet
    let project_root = PathBuf::from(project_path(project_slug));
    let project_root = tokio::fs::canonicalize(&project_root)
        .await
        .map_err(ServerError::from)?;

    // 2) Rejeter tout chemin absolu ou contenant `..`
    let rel = PathBuf::from(user_path);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ServerError::InvalidPath.into());
    }

    // Chemin final (peut ne pas exister)
    let full_path = project_root.join(rel);

    if should_exist {
        // 3A) On attend que la cible existe → canonicaliser puis métadonnées
        let canon = tokio::fs::canonicalize(&full_path)
            .await
            .map_err(ServerError::from)?;

        // 4A) Vérifier qu’elle reste sous project_root
        if !canon.starts_with(&project_root) {
            return Err(ServerError::OutOfProjectsScope.into());
        }

        // 5A) Vérifier fichier vs dossier
        let meta = tokio::fs::metadata(&canon)
            .await
            .map_err(ServerError::from)?;
        if is_file && !meta.is_file() {
            return Err(ServerError::PathIsNotFile.into());
        }
        if !is_file && !meta.is_dir() {
            return Err(ServerError::PathIsNotDir.into());
        }

        Ok(canon)
    } else {
        // 3B) Création de la cible → vérifier uniquement le parent
        let parent = full_path
            .parent()
            .ok_or(ServerError::PathHasNoParent)?;
        let parent_canon = tokio::fs::canonicalize(parent)
            .await
            .map_err(ServerError::from)?;

        // 4B) S’assurer que le parent est dans le projet
        if !parent_canon.starts_with(&project_root) {
            return Err(ServerError::OutOfProjectsScope.into());
        }

        // 5B) OK pour créer : retourner le chemin (non-canon) où l’on créera.
        Ok(full_path)
    }
}
