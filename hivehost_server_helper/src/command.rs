use crate::{AppState, BTRFS_DEVICE, ServerHelperResult};
use common::command::run_external_command;
use secrecy::ExposeSecret;

use common::helper_command::tarpc::ServerHelper;
use common::helper_command::{HelperCommand, HelperResponse};
use common::{
    AuthResponse, AuthToken, SERVICE_USER, USER_GROUP, Validate, get_project_dev_path,
    get_project_prod_path, get_project_snapshot_path, get_user_path, get_user_project_path,
    get_user_projects_path,
};
use tarpc::context::Context;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::info;

#[derive(Clone)]
pub struct ServerHelperServer(pub AppState);

impl ServerHelper for ServerHelperServer {
    async fn execute(self, _: Context, actions: Vec<HelperCommand>) -> HelperResponse {
        info!("Helper actions: {:?}", actions);
        for action in actions {
            if let Err(e) = execute_command(action).await {
                tracing::error!("Error executing command: {}", e);
                return HelperResponse::Error(e.to_string());
            }
        }
        HelperResponse::Ok
    }

    async fn auth(self, _: Context, token: AuthToken) -> AuthResponse {
        let mut connected = self.0.connected.write().await;
        if self.0.server_auth.expose_secret().eq(&token.0) {
            info!("Token auth success");
            *connected = true;
            AuthResponse::Ok
        } else {
            *connected = false;
            info!("Token auth failed");
            AuthResponse::Error
        }
    }
}

pub async fn execute_command(action: HelperCommand) -> ServerHelperResult<()> {
    action.validate()?;
    match action {
        HelperCommand::CreateUser { user_slug } => {
            let user_path = get_user_path(&user_slug);
            let user_projects_path = get_user_projects_path(&user_slug);
            run_external_command(
                "useradd",
                &[
                    "--system",
                    "--gid",
                    USER_GROUP,
                    "--home-dir",
                    &user_path,
                    "--create-home",
                    "--shell",
                    "/usr/sbin/nologin",
                    &user_slug.0,
                ],
            )
            .await?;

            run_external_command("chown", &["root:root", &user_path]).await?;
            run_external_command("chmod", &["755", &user_path]).await?;
            run_external_command("mkdir", &["-p", &user_projects_path]).await?;
            run_external_command("chown", &["root:root", &user_projects_path]).await?;
            run_external_command("chmod", &["755", &user_projects_path]).await?;
        }
        HelperCommand::DeleteUser { user_slug } => {
            let user_path = get_user_path(&user_slug);
            run_external_command("userdel", &["--remove", &user_slug.0]).await?;
            run_external_command("rm", &["-rf", &user_path]).await?;
        }
        HelperCommand::CreateProject {
            project_slug,
            user_slug,
            with_index_html,
        } => {
            let dev_path = get_project_dev_path(&project_slug);
            let prod_path = get_project_prod_path(&project_slug);

            run_external_command("btrfs", &["subvolume", "create", &dev_path]).await?;
            run_external_command("chown", &["root:root", &dev_path]).await?;
            run_external_command("chmod", &["700", &dev_path]).await?;

            let user_acl_rwx_entry = format!("u:{}:rwX", user_slug.0);
            let service_acl_rwx_entry = format!("u:{SERVICE_USER}:rwX");

            run_external_command("setfacl", &["-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-m", &service_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &service_acl_rwx_entry, &dev_path])
                .await?;

            run_external_command("mkdir", &["-p", &prod_path]).await?;
            run_external_command("chown", &["root:root", &prod_path]).await?;
            run_external_command("chmod", &["755", &prod_path]).await?;

            if with_index_html {
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
                index_file.flush().await?
            }
        }
        HelperCommand::DeleteProject { project_slug } => {
            let project_path = get_project_dev_path(&project_slug);
            let prod_path = get_project_prod_path(&project_slug);
            run_external_command("rm", &["-rf", &prod_path]).await?;
            run_external_command("btrfs", &["subvolume", "delete", &project_path]).await?;
        }
        HelperCommand::SetAcl {
            project_slug,
            user_slug,
            is_read_only,
        } => {
            let path = get_project_dev_path(&project_slug);
            let perms = if is_read_only { "r-X" } else { "rwX" };
            let acl_entry = format!("u:{}:{perms}", user_slug.0);
            run_external_command("setfacl", &["-R", "-m", &acl_entry, &path]).await?;
            run_external_command("setfacl", &["-d", "-m", &acl_entry, &path]).await?;
        }
        HelperCommand::RemoveAcl {
            project_slug,
            user_slug,
        } => {
            let path = get_project_dev_path(&project_slug);
            let acl_spec = format!("u:{}", user_slug.0);
            run_external_command("setfacl", &["-x", &acl_spec, &path]).await?;
            run_external_command("setfacl", &["-d", "-x", &acl_spec, &path]).await?;
        }
        HelperCommand::BindMountUserProject {
            project_slug,
            user_slug,
        } => {
            let project_path = get_project_dev_path(&project_slug);
            let user_project_path = get_user_project_path(&user_slug, &project_slug);
            run_external_command("mkdir", &["-p", &user_project_path]).await?;
            run_external_command("mount", &["--bind", &project_path, &user_project_path]).await?;
        }
        HelperCommand::UnmountUserProject {
            project_slug,
            user_slug,
        } => {
            let user_project_path = get_user_project_path(&user_slug, &project_slug);
            let r = run_external_command(
                "findmnt",
                &["-n", "-o", "TARGET", "--target", &user_project_path],
            )
            .await?;
            if !r.is_empty() && !r.eq("/")  {
                run_external_command("umount", &[&user_project_path]).await?;
            }

            tokio::fs::remove_dir(&user_project_path).await?;
        }
        HelperCommand::CreateSnapshot {
            project_slug,
            snapshot_name,
        } => {
            let path = get_project_dev_path(&project_slug);
            let snapshot_path = get_project_snapshot_path(&snapshot_name.0);
            run_external_command(
                "btrfs",
                &["subvolume", "snapshot", "-r", &path, &snapshot_path],
            )
            .await?;
        }
        HelperCommand::DeleteSnapshot { snapshot_name } => {
            let snapshot_path = get_project_snapshot_path(&snapshot_name.0);
            run_external_command("btrfs", &["subvolume", "delete", &snapshot_path]).await?;
        }
        HelperCommand::MountSnapshot {
            project_slug,
            snapshot_name,
        } => {
            let path = get_project_prod_path(&project_slug);
            run_external_command(
                "mount",
                &[
                    "-o",
                    &format!("subvol={},ro", snapshot_name.0),
                    BTRFS_DEVICE.as_str(),
                    &path,
                ],
            )
            .await?;
        }
        HelperCommand::UnmountProd { project_slug } => {
            let path = get_project_prod_path(&project_slug);
            let r = run_external_command(
                "findmnt",
                &["-n", "-o", "TARGET", "--target", &path],
            ).await?;
            if !r.is_empty() && !r.eq("/") {
                run_external_command("umount", &[&path]).await?;
            }
        }
        HelperCommand::RestoreSnapshot {
            project_slug,
            snapshot_name,
        } => {
            let path = get_project_dev_path(&project_slug);
            let snapshot_path = get_project_snapshot_path(&snapshot_name.0);
            run_external_command("btrfs", &["subvolume", "snapshot", &snapshot_path, &path])
                .await?;
        }
    }
    Ok(())
}
