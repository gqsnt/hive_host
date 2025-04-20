use std::path::PathBuf;
use crate::{ensure_authorization, AppState, ServerProjectId};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use common::server_project_action::{
    IsProjectServerAction, ServerProjectAction, ServerProjectActionRequest,
    ServerProjectActionResponse,
};
use common::{ProjectId, ProjectUnixSlugStr};
use secrecy::ExposeSecret;
use common::server_project_action::io_action::dir_action::{DirAction, DirActionLsResponse, LsElement};
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::io_action::IoAction;
use common::server_project_action::permission::PermissionAction;
use crate::cmd::{project_path, set_acl};
use crate::server_action::{add_user_to_project, remove_user_from_project, update_user_in_project};

pub async fn project_action_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    if let Some((project_slug, action)) = state.server_project_action_cache.get(&token).await {
        state.server_project_action_cache.invalidate(&token).await;
        tracing::debug!("Token match action {:?}", action);
        handle_server_project_action(state, project_slug, action).await
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
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
            handle_server_project_action(state, request.project_slug.to_unix(), request.action).await
        }
    })
}

pub async fn handle_server_project_action(
    state: AppState,
    project_slug: ProjectUnixSlugStr,
    action: ServerProjectAction,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action{
        ServerProjectAction::Io(io) => handle_server_project_action_io(project_slug, io).await,
        ServerProjectAction::Permission(permission) => handle_server_project_action_permission(project_slug, permission).await,
    }
}


pub async fn handle_server_project_action_permission(
    project_slug: ProjectUnixSlugStr,
    action: PermissionAction,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        PermissionAction::Grant { user_slug, permission } => {
            if let Err(e) = add_user_to_project(
                user_slug.to_unix(),
                project_slug,
                permission,
            ).await{
                tracing::debug!("Error adding user to project: {:?}", e);
            }
        }
        PermissionAction::Revoke { user_slug } => {
            if let Err(e) = remove_user_from_project(
                user_slug.to_unix(),
                project_slug
            ).await{
                tracing::debug!("Error removing user from project: {:?}", e);
            }
        }
        PermissionAction::Update { user_slug, permission } => {
            if let Err(e) = update_user_in_project(
                user_slug.to_unix(),
                project_slug,
                permission,
            ).await{
                tracing::debug!("Error updating user in project: {:?}", e);
            }
        }
    }
    Ok(Json(ServerProjectActionResponse::Ok))
}
pub async fn handle_server_project_action_io(
    project_slug: ProjectUnixSlugStr,
    action: IoAction,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        IoAction::Dir(dir) => handle_server_project_action_dir(project_slug, dir).await,
        IoAction::File(file) => handle_server_project_action_file(project_slug, file).await,
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
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de créer le répertoire : {}", e),
                    )
                })?;
        }
        DirAction::Rename { path, new_name } => {
            let path = ensure_path_in_project_path(project_slug.clone(), &path, false, true).await?;
            let new_name = ensure_path_in_project_path(project_slug.clone(), &new_name, false, false).await?;
            tokio::fs::rename(path, new_name)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de renommer le répertoire : {}", e),
                    )
                })?;
        }
        DirAction::Delete { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, true).await?;
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de supprimer le répertoire : {}", e),
                    )
                })?;
        }
        DirAction::Ls { path } => {
            let path = ensure_path_in_project_path(project_slug, &path, false, true).await?;
            let mut entries = tokio::fs::read_dir(path)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de lire le répertoire : {}", e),
                    )
                })?;
            let mut result = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await{
                let meta = entry.metadata().await.unwrap();
                result.push(LsElement {
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: meta.is_dir(),
                })
            }
            return Ok(Json(ServerProjectActionResponse::Ls(DirActionLsResponse{inner: result})));
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
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de créer l'archive : {}", e),
                    )
                })?;
            if !output.status.success() {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Impossible de créer l'archive : {}", String::from_utf8_lossy(&output.stderr)),
                ));
            }
            let tar_file = tokio::fs::File::open(tar_path)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de lire l'archive : {}", e),
                    )
                })?;
            let mut reader = tokio::io::BufReader::new(tar_file);
            // return the file to the client
            let mut buf = Vec::new();
            tokio::io::copy(&mut reader, &mut buf)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Impossible de lire l'archive : {}", e),
                    )
                })?;
            return Ok(Json(ServerProjectActionResponse::Content(String::from_utf8(buf).unwrap())));
        }
    }
    Ok(Json(ServerProjectActionResponse::Ok))
}
pub async fn handle_server_project_action_file(
    project_slug: ProjectUnixSlugStr,
    action: FileAction,
) -> Result<Json<ServerProjectActionResponse>, (StatusCode, String)> {
    match action {
        FileAction::Create { path, name, content } => {}
        FileAction::Rename { path, new_name } => {}
        FileAction::Delete { path } => {}
        FileAction::Move { path, new_path } => {}
        FileAction::Copy { path, new_path } => {}
        FileAction::View { path } => {}
        FileAction::Update { path, content } => {}
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
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Impossible de résoudre la racine projet : {}", e),
            )
        })?;

    // 2) Rejeter tout chemin absolu ou contenant `..`
    let rel = PathBuf::from(user_path);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err((StatusCode::BAD_REQUEST, format!("Chemin invalide : {}", user_path)));
    }

    // Chemin final (peut ne pas exister)
    let full_path = project_root.join(rel);

    if should_exist {
        // 3A) On attend que la cible existe → canonicaliser puis métadonnées
        let canon = tokio::fs::canonicalize(&full_path).await.map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("La cible '{}' n’existe pas : {}", user_path, e),
            )
        })?;

        // 4A) Vérifier qu’elle reste sous project_root
        if !canon.starts_with(&project_root) {
            return Err((
                StatusCode::FORBIDDEN,
                format!("Accès hors projet : {}", user_path),
            ));
        }

        // 5A) Vérifier fichier vs dossier
        let meta = tokio::fs::metadata(&canon).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Impossible de lire les méta : {}", e),
            )
        })?;
        if is_file && !meta.is_file() {
            return Err((StatusCode::BAD_REQUEST, format!("'{}' n’est pas un fichier", user_path)));
        }
        if !is_file && !meta.is_dir() {
            return Err((StatusCode::BAD_REQUEST, format!("'{}' n’est pas un dossier", user_path)));
        }

        Ok(canon)
    } else {
        // 3B) Création de la cible → vérifier uniquement le parent
        let parent = full_path.parent().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("Chemin invalide, pas de répertoire parent : {}", user_path),
            )
        })?;
        let parent_canon = tokio::fs::canonicalize(parent).await.map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Le dossier parent '{}' n’existe pas : {}", parent.display(), e),
            )
        })?;

        // 4B) S’assurer que le parent est dans le projet
        if !parent_canon.starts_with(&project_root) {
            return Err((
                StatusCode::FORBIDDEN,
                format!("Parent hors projet : {}", parent.display()),
            ));
        }

        // 5B) OK pour créer : retourner le chemin (non-canon) où l’on créera.
        Ok(full_path)
    }
}
