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
