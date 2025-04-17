use std::path::PathBuf;
use crate::{ensure_authorization, AppState, ServerProjectId};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use common::server_project_action::{
    IsProjectServerAction, ServerProjectAction, ServerProjectActionRequest,
    ServerProjectActionResponse,
};
use common::{ProjectId, ProjectUnixSlugStr};
use secrecy::ExposeSecret;
use crate::cmd::project_path;

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
    Ok(Json(ServerProjectActionResponse::Content(
        "SUPER SECRET MESSAGE IN A FILE".to_string(),
    )))
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
