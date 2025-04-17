use crate::cmd::{bind_project_to_user_chroot, ensure_user_in_sshd, ensure_user_removed_in_sshd, project_path, run_sudo_cmd, set_acl, ssh_key_path, ssh_path, user_path, user_project_path, user_projects_path};
use crate::{ensure_authorization, AppState, ServerProjectId, ServerUserId};
use axum::extract::{State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use common::permission::Permission;
use common::server_action::user_action::UserAction;
use common::server_action::{ServerAction, ServerActionResponse};
use common::{ProjectId, ProjectUnixSlugStr, UserId, UserSlug, UserUnixSlugStr};
use secrecy::ExposeSecret;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

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
    match action {
        ServerAction::UserAction(user_action) => match user_action {
            UserAction::Create {
                user_slug,
            } => match create_user(user_slug.to_unix()).await {
                Ok(_) => Ok(Json(ServerActionResponse::Ok)),
                Err(e) => {
                    tracing::debug!("Error creating user: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Error creating user".to_string(),
                    ))
                }
            },
            UserAction::AddSshKey { user_slug, ssh_key } => {
                match add_ssh_key(user_slug.to_unix(), ssh_key).await {
                    Ok(_) => Ok(Json(ServerActionResponse::Ok)),
                    Err(e) => {
                        tracing::debug!("Error adding ssh key: {:?}", e);
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error adding ssh key".to_string(),
                        ))
                    }
                }
            }
            UserAction::Delete { user_slug } => match remove_user(user_slug.to_unix()).await {
                Ok(_) => Ok(Json(ServerActionResponse::Ok)),
                Err(e) => {
                    tracing::debug!("Error removing user: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Error removing user".to_string(),
                    ))
                }
            },
            UserAction::AddProject { user_slug, project_slug } => {
                match create_project(user_slug.to_unix(), project_slug.to_unix()).await {
                    Ok(_) => Ok(Json(ServerActionResponse::Ok)),
                    Err(e) => {
                        tracing::debug!("Error creating project: {:?}", e);
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error creating project".to_string(),
                        ))
                    }
                }
            }
        },
    }
}

pub async fn create_user(user_slug: UserUnixSlugStr) -> Result<(), tokio::io::Error> {
    let user_path_str = user_path(user_slug.clone());
    let user_projects_path_str = user_projects_path(user_slug.clone());
    tracing::debug!("Creating user {}", &user_slug);
    tokio::fs::create_dir_all(&user_projects_path_str).await?;
    run_sudo_cmd(

        &["useradd","-d", &user_path_str, "-s", "/usr/sbin/nologin", &user_slug],
    )
    .await?;

    run_sudo_cmd( &["chown","root:root", &user_path_str]).await?;
    run_sudo_cmd( &["chmod","755", &user_path_str]).await?;

    run_sudo_cmd(
        &["chown",&format!("{0}:{0}", user_slug), &user_projects_path_str],
    )
    .await?;

    let ssh_path = ssh_path(user_slug.clone());
    let ssh_key_path = ssh_key_path(user_slug.clone());
    tokio::fs::create_dir_all(&ssh_path).await?;
    let ssh_key_file = tokio::fs::File::create(&ssh_key_path).await?;
    run_sudo_cmd( &["chown","-R", "root:root", &user_path_str]).await?;
    run_sudo_cmd( &["chown","-R", &format!("{0}:{0}", &user_slug), &ssh_path]).await?;
    run_sudo_cmd( &["chmod","700", &ssh_path]).await?;
    run_sudo_cmd( &["chmod","600", &ssh_key_path]).await?;
    ensure_user_in_sshd(user_slug).await?;
    Ok(())
}

pub async fn create_project(
    user_slug: UserUnixSlugStr,
    project_slug:ProjectUnixSlugStr,
) -> Result<(), tokio::io::Error> {
    tracing::debug!("Creating project: {:?}", &project_slug);
    let proj_path = project_path(project_slug.clone());

    tokio::fs::create_dir_all(&proj_path).await?;
    run_sudo_cmd( &["chown","root:root", &proj_path]).await?;
    run_sudo_cmd( &["chmod","700", &proj_path]).await?;
    set_acl(&proj_path, user_slug.clone(), "rwX").await?;

    bind_project_to_user_chroot(user_slug,project_slug).await?;
    Ok(())
}

pub async fn add_user_to_project(
    user_slug: UserUnixSlugStr,
    project_slug:ProjectUnixSlugStr,
    permission: Permission,
) -> Result<(), tokio::io::Error> {
    let acl = if permission >= Permission::Write {
        "rwX"
    } else {
        "r-X"
    };
    let proj_path = project_path(project_slug.clone());
    set_acl(&proj_path, user_slug.clone(), acl).await?;
    bind_project_to_user_chroot(user_slug, project_slug).await?;

    Ok(())
}

pub async fn remove_user_from_project(
    user_slug: UserUnixSlugStr,
    project_slug:ProjectUnixSlugStr,
) -> Result<(), tokio::io::Error> {
    let proj_path = project_path(project_slug.clone());

    run_sudo_cmd( &["setfacl","-x", &format!("u:{}", &user_slug), &proj_path]).await?;
    run_sudo_cmd(

        &["setfacl","-d", "-x", &format!("u:{}", &user_slug), &proj_path],
    )
    .await?;

    run_sudo_cmd( &["umount",&user_project_path(user_slug, project_slug)]).await?;

    Ok(())
}

pub async fn add_ssh_key(user_slug: UserUnixSlugStr, ssh_key: String) -> Result<(), tokio::io::Error> {
    let ssh_key_file_path =  ssh_key_path(user_slug.clone());
    let ssh_key_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ssh_key_file_path)
        .await?;
    let mut file = tokio::io::BufWriter::new(ssh_key_file);
    file.write_all(ssh_key.as_bytes()).await?;
    file.write_all(b"\n").await?;
    file.flush().await?;
    Ok(())
}

pub async fn remove_project(project_slug:ProjectUnixSlugStr) -> Result<(), std::io::Error> {
    let proj_path = project_path(project_slug);
    run_sudo_cmd( &["umount",&proj_path]).await.ok(); // ignore errors
    tokio::fs::remove_dir_all(&proj_path).await?;
    Ok(())
}

pub async fn remove_user(user_slug: UserUnixSlugStr) -> Result<(), tokio::io::Error> {
    let user_projects_path_str = user_projects_path(user_slug.clone());
    let path = Path::new(&user_projects_path_str);

    if path.exists() {
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let submount = entry.path();
            if submount.is_dir() {
                run_sudo_cmd( &["umount",submount.to_str().unwrap()]).await?;
            }
        }
    }
    ensure_user_removed_in_sshd(user_slug.clone()).await?;
    run_sudo_cmd( &["userdel",&user_slug]).await?;
    tokio::fs::remove_dir_all(user_path(user_slug)).await?;

    Ok(())
}
