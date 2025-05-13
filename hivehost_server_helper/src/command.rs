use crate::{ServerHelperResult, BTRFS_DEVICE};
use common::command::run_external_command;

use common::{get_project_dev_path, get_project_prod_path, SERVICE_USER, USER_GROUP};
use tarpc::context::Context;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::info;
use common::helper_command::{HelperCommand, HelperResponse};
use common::helper_command::tarpc::ServerHelper;

#[derive(Clone)]
pub struct ServerHelperServer;

impl ServerHelper for ServerHelperServer {
    async fn execute(self, _: Context, actions: Vec<HelperCommand>) -> HelperResponse {
        info!("Helper actions: {:?}", actions);
        for action in actions {
            if let Err(e) =  execute_command(action).await{
                tracing::error!("Error executing command: {}", e);
                return HelperResponse::Error(e.to_string());
            }
        }
        HelperResponse::Ok
    }
}

pub async fn execute_command(
    action: HelperCommand,
) -> ServerHelperResult<()> {
    match action {
        HelperCommand::CreateUser {
            user_slug,
            user_path,
            user_projects_path,
        } => {
            // 1. Create user
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
                    &user_slug,
                ],
            )
            .await?;

            run_external_command("chown", &["root:root", &user_path]).await?;
            run_external_command("chmod", &["755", &user_path]).await?;
            run_external_command("mkdir", &["-p", &user_projects_path]).await?;
            run_external_command("chown", &["root:root", &user_projects_path]).await?;
            run_external_command("chmod", &["755", &user_projects_path]).await?;
        }
        HelperCommand::DeleteUser { user_slug, user_path } => {
            run_external_command("userdel", &["--remove", &user_slug]).await?;
            run_external_command("rm", &["-rf", &user_path]).await?;
        }
        HelperCommand::CreateProject {
            project_slug: project_slug_str,
            service_user,
            with_index_html,
        } => {
            let dev_path = get_project_dev_path(&project_slug_str);
            let prod_path = get_project_prod_path(&project_slug_str);

            // 1. Create the main project Btrfs subvolume
            run_external_command("btrfs", &["subvolume", "create", &dev_path]).await?;
            run_external_command("chown", &["root:root", &dev_path]).await?;
            run_external_command("chmod", &["700", &dev_path]).await?; // Start restrictive

            let user_acl_rwx_entry = format!("u:{service_user}:rwX");
            let service_acl_rwx_entry = format!("u:{SERVICE_USER}:rwX");

            run_external_command("setfacl", &["-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-m", &service_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &service_acl_rwx_entry, &dev_path]).await?;

            run_external_command("mkdir", &["-p", &prod_path]).await?;
            run_external_command("chown", &["root:root", &prod_path]).await?;
            run_external_command("chmod", &["755", &prod_path]).await?;
            if with_index_html{
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
        HelperCommand::DeleteProject {
            project_slug: project_slug_str,
        } => {
            let project_path = get_project_dev_path(&project_slug_str);
            let prod_path = get_project_prod_path(&project_slug_str);
            run_external_command("rm", &["-rf", &prod_path]).await?;
            run_external_command("btrfs", &["subvolume", "delete", &project_path]).await?;
        }
        HelperCommand::SetAcl {
            path,
            user_slug,
            is_read_only,
        } => {
            let perms = if is_read_only { "r-X" } else { "rwX" };
            let acl_entry = format!("u:{user_slug}:{perms}");
            run_external_command("setfacl", &["-R", "-m", &acl_entry, &path]).await?;
            run_external_command("setfacl", &["-d", "-m", &acl_entry, &path]).await?;
        }
        HelperCommand::RemoveAcl { path, user_slug } => {
            let acl_spec = format!("u:{user_slug}");
            run_external_command("setfacl", &["-x", &acl_spec, &path]).await?;
            run_external_command("setfacl", &["-d", "-x", &acl_spec, &path]).await?;
        }
        HelperCommand::BindMountUserProject {
            source_path,
            target_path,
        } => {
            run_external_command("mkdir", &["-p", &target_path]).await?;
            // run_external_command("chown", &["root:root", &target_path]).await?;
            // run_external_command("chmod", &["755", &target_path]).await?;
            run_external_command("mount", &["--bind", &source_path, &target_path]).await?;
        }
        HelperCommand::UnmountUserProject { target_path } => {
            // Check if it's actually mounted before trying to unmount?
            // `findmnt -n -o TARGET --target "$target_path"`
            run_external_command("umount", &[&target_path]).await?;
            // Attempt to remove the empty mount point directory after unmounting
            tokio::fs::remove_dir(&target_path).await?;
        }
        HelperCommand::CreateSnapshot {
            path,
            snapshot_path,
        } => {
            run_external_command(
                "btrfs",
                &[
                    "subvolume",
                    "snapshot",
                    "-r", // Read-only flag
                    &path,
                    &snapshot_path,
                ],
            )
            .await?;
        }
        HelperCommand::DeleteSnapshot { snapshot_path } => {
            run_external_command("btrfs", &["subvolume", "delete", &snapshot_path]).await?;
        }
        HelperCommand::MountSnapshot {
            path,
            snapshot_name,
        } => {
            run_external_command(
                "mount",
                &[
                    "-o",
                    &format!("subvol={snapshot_name},ro"),
                    BTRFS_DEVICE.as_str(),
                    &path,
                ],
            )
            .await?;
        }
        HelperCommand::UnmountProd { path } => {
            run_external_command("umount", &[&path]).await?;
        }
        HelperCommand::RestoreSnapshot { path, snapshot_path, users_project_path } => {
            for user_project_path in &users_project_path {
                run_external_command(
                    "umount",
                    &[
                        user_project_path,
                    ],
                )
                .await?;
            }
            run_external_command(
                "btrfs",
                &[
                    "subvolume",
                    "delete",
                    &path,
                ],
            )
            .await?;
            run_external_command(
                "btrfs",
                &[
                    "subvolume",
                    "snapshot",
                    &snapshot_path,
                    &path,
                ],
            )
            .await?;
            for user_project_path in &users_project_path {
                run_external_command("mount", &["--bind", &path, user_project_path]).await?;
            }
        }
    }
    Ok(())
}
