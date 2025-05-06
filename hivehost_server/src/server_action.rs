use common::website_to_server::permission::Permission;
use common::website_to_server::server_action::user_action::ServerUserAction;
use common::website_to_server::server_action::{ServerAction, ServerActionResponse};
use common::{
    get_project_dev_path, get_user_path, get_user_project_path, get_user_projects_path,
    ProjectSlugStr, UserSlugStr,
};
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::info;
use common::server::server_to_helper::{ServerToHelperAction};
use crate::{AppState, ServerResult, TarpcHelperClient};

pub async fn handle_server_action(
    state: AppState,
    action: ServerAction,
) -> ServerResult<ServerActionResponse> {
    info!("Server action: {:?}", action);
    match action {
        ServerAction::UserAction(user_action) => match user_action {
            ServerUserAction::Create { user_slug } => {
                create_user(state.helper_client, user_slug.to_string()).await?;
            }
            ServerUserAction::Delete { user_slug } => {
                remove_user(state.helper_client, user_slug.to_string()).await?;
            }
            ServerUserAction::AddProject {
                user_slug,
                project_slug,
            } => {
                create_project(
                    state.helper_client,
                    user_slug.to_string(),
                    project_slug.to_string(),
                )
                .await?;
                
            }
            ServerUserAction::RemoveProject {
                user_slugs,
                project_slug,
            } => {
                for user_slug in user_slugs {
                    remove_user_from_project(
                        state.helper_client.clone(),
                        user_slug.to_string(),
                        project_slug.to_string(),
                    )
                    .await?;
                }
                remove_project(state.helper_client, project_slug.to_string()).await?;
            }
        },
    }
    Ok(ServerActionResponse::Ok)
}

pub async fn create_user(helper_client: TarpcHelperClient, user_slug: UserSlugStr) -> ServerResult<()> {
    let user_path = get_user_path(&user_slug);
    let user_projects_path = get_user_projects_path(&user_slug);
    helper_client.execute(|c, cx|async move {
        c.execute(cx, ServerToHelperAction::CreateUser {
            user_slug,
            user_path: user_path.clone(),
            user_projects_path: user_projects_path.clone(),
        }).await
    }).await?;
    Ok(())
}

pub async fn create_project(
    helper_client: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) -> ServerResult<()> {
    let project_slug_clone = project_slug.clone();
    let user_slug_clone = user_slug.clone();
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::CreateProject {
            project_slug: project_slug_clone,
            service_user: user_slug_clone,
        }).await
    }).await?;

    let dev_path = get_project_dev_path(&project_slug);

    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    let dev_path_clone = dev_path.clone();
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::BindMountUserProject {
            source_path: dev_path_clone,
            target_path: user_project_path.clone(),
        }).await
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
    helper_client: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<()> {
    let project_path = get_project_dev_path(&project_slug);
    let project_path_clone = project_path.clone();
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::SetAcl {
            path:project_path_clone,
            user_slug,
            is_read_only: permission.is_read_only(),
        }).await
    }).await?;
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::BindMountUserProject {
            source_path: project_path,
            target_path: user_project_path,
        }).await
    }).await?;
    
    Ok(())
}

pub async fn update_user_in_project(
    helper_client: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<()> {
    helper_client.execute(|c, cx|async move {
        c.execute(cx, ServerToHelperAction::SetAcl {
            path: get_project_dev_path(&project_slug),
            user_slug,
            is_read_only: permission.is_read_only(),
        }).await
    }).await?;
    Ok(())
}

pub async fn remove_user_from_project(
    helper_client: TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) -> ServerResult<()> {
    let proj_path = get_project_dev_path(&project_slug);
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::RemoveAcl {
            path: proj_path.clone(),
            user_slug: user_slug.clone(),
        }).await
    }).await?;
    
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::UnmountUserProject {
            target_path: user_project_path.clone(),
        }).await
    }).await?;
    Ok(())
}

pub async fn remove_project(
    helper_client: TarpcHelperClient,
    project_slug: ProjectSlugStr,
) -> ServerResult<()> {
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::DeleteProject { project_slug }).await
    }).await?;
    Ok(())
}

pub async fn remove_user(helper_client: TarpcHelperClient, user_slug: UserSlugStr) -> ServerResult<()> {
    let user_projects_path = get_user_projects_path(&user_slug);
    let path = Path::new(&user_projects_path);

    if path.exists() {
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let submount = entry.path();
            if submount.is_dir() {

                helper_client.execute(|c, cx|async move {
                    c.execute(cx,ServerToHelperAction::UnmountUserProject {
                        target_path: submount.to_string_lossy().to_string(),
                    }).await
                }).await?;
            }
        }
    }
    let user_slug_clone = user_slug.clone();
    helper_client.execute(|c, cx|async move {
        c.execute(cx,ServerToHelperAction::DeleteUser {
            user_slug: user_slug_clone,
        }).await
    }).await?;
    tokio::fs::remove_dir_all(get_user_path(&user_slug)).await?;

    Ok(())
}
