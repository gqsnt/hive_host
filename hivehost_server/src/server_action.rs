use crate::ServerError;
use crate::{ensure_authorization, AppState, ServerResult};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use common::permission::Permission;
use common::server_action::user_action::UserAction;
use common::server_action::{ServerAction, ServerActionResponse};
use common::{get_project_dev_path, get_user_path, get_user_project_path, get_user_projects_path, ProjectSlugStr, UserSlugStr};
use secrecy::ExposeSecret;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::info;
use common::server_helper::ServerHelperCommand;
use crate::helper_client::HelperClient;

pub async fn server_action(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ServerAction>,
) -> Result<Json<ServerActionResponse>, (StatusCode, String)> {
    ensure_authorization!(headers, state, {
        handle_server_action(state, request).await
    })
}
pub async fn handle_server_action(
    state: AppState,
    action: ServerAction,
) -> Result<Json<ServerActionResponse>, (StatusCode, String)> {
    info!("Server action: {:?}", action);
    match action {
        ServerAction::UserAction(user_action) => match user_action {
            UserAction::Create { user_slug } => {
                create_user(state.helper_client, user_slug.to_string()).await?;
                Ok(Json(ServerActionResponse::Ok))
            }
            UserAction::Delete { user_slug } => {
                remove_user(state.helper_client, user_slug.to_string()).await?;
                Ok(Json(ServerActionResponse::Ok))
            }
            UserAction::AddProject {
                user_slug,
                project_slug,
            } => {
                create_project(state.helper_client, user_slug.to_string(), project_slug.to_string()).await?;
                Ok(Json(ServerActionResponse::Ok))
            }
            UserAction::RemoveProject {
                user_slugs,
                project_slug,
            } => {
                for user_slug in user_slugs {
                    remove_user_from_project(state.helper_client.clone(), user_slug.to_string(), project_slug.to_string()).await?;
                }
                remove_project(state.helper_client, project_slug.to_string()).await?;
                Ok(Json(ServerActionResponse::Ok))
            }
        },
    }
}

pub async fn create_user(helper_client:HelperClient, user_slug: UserSlugStr) -> ServerResult<()> {
    let user_path = get_user_path(&user_slug);
    let user_projects_path = get_user_projects_path(&user_slug);
    helper_client.execute(ServerHelperCommand::CreateUser {
        user_slug,
        user_path:user_path.clone(),
        user_projects_path: user_projects_path.clone(),
    }).await?;
    Ok(())
}

pub async fn create_project(
    helper_client:HelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) -> ServerResult<()> {
    helper_client.execute(ServerHelperCommand::CreateProject {
        project_slug:project_slug.clone(),
        service_user: user_slug.clone(),
    }).await?;
    
    
    let dev_path = get_project_dev_path(&project_slug);
    
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    helper_client.execute(ServerHelperCommand::BindMountUserProject {
        source_path: dev_path.clone(),
        target_path: user_project_path.clone(),
    }).await?;
    //add file index.html to project
    let index_file_path = format!("{dev_path}/index.html");
    let mut index_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&index_file_path)
        .await?;
    index_file
        .write_all(b"<html><body><h1>Hello World</h1></body></html>")
        .await?;
    index_file.flush().await?;
    Ok(())
}

pub async fn add_user_to_project(
    helper_client:HelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<()> {
    
    let is_read_only = permission < Permission::Write;
    let project_path = get_project_dev_path(&project_slug); 
    helper_client.execute(ServerHelperCommand::SetAcl {
        path: project_path.clone(),
        user_slug:user_slug.clone(),
        is_read_only,
    }).await?;
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    helper_client.execute(ServerHelperCommand::BindMountUserProject {
        source_path: project_path,
        target_path: user_project_path,
    }).await?;
    Ok(())
}

pub async fn update_user_in_project(
    helper_client:HelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<()> {
    let is_read_only = permission < Permission::Write;
    helper_client.execute(ServerHelperCommand::SetAcl {
        path: get_project_dev_path(&project_slug),
        user_slug,
        is_read_only,
    }).await?;
    Ok(())
}

pub async fn remove_user_from_project(
    helper_client:HelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) -> ServerResult<()> {
    let proj_path = get_project_dev_path(&project_slug);
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    helper_client.execute(ServerHelperCommand::RemoveAcl {
        path: proj_path.clone(),
        user_slug:user_slug.clone(),
    }).await?;
    
    helper_client.execute(ServerHelperCommand::UnmountUserProject {
        target_path: user_project_path.clone(),
    }).await?;
    Ok(())
}


pub async fn remove_project(helper_client:HelperClient,project_slug: ProjectSlugStr) -> ServerResult<()> {
    helper_client.execute(ServerHelperCommand::DeleteProject {
        project_slug,
    }).await?;
    Ok(())
}

pub async fn remove_user(helper_client:HelperClient,user_slug: UserSlugStr) -> ServerResult<()> {
    let user_projects_path = get_user_projects_path(&user_slug);
    let path = Path::new(&user_projects_path);

    if path.exists() {
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let submount = entry.path();
            if submount.is_dir() {
                helper_client.execute(ServerHelperCommand::UnmountUserProject {
                    target_path: submount.to_string_lossy().to_string(),
                }).await?;
            }
        }
    }
    helper_client.execute(ServerHelperCommand::DeleteUser {
        user_slug:user_slug.clone(),
    }).await?;
    tokio::fs::remove_dir_all(get_user_path(&user_slug)).await?;

    Ok(())
}
