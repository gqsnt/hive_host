
use tracing::{ info};
use common::command::run_external_command;
use common::{get_project_dev_path, get_project_prod_path, SERVICE_USER, USER_GROUP};
use common::server_helper::ServerHelperCommand;
use crate::{ServerHelperResult, BTRFS_DEVICE};


pub async fn execute_command(command: ServerHelperCommand) -> ServerHelperResult<()> {
    info!("Executing command: {:?}", command);
    match command {
        ServerHelperCommand::CreateUser { user_slug, user_path, user_projects_path } => {
            // 1. Create user
            run_external_command("useradd", &[
                "--system",
                "--gid", USER_GROUP,
                "--home-dir", &user_path,
                "--create-home",
                "--shell", "/usr/sbin/nologin",
                &user_slug,
            ]).await?;

            run_external_command("chown", &["root:root", &user_path]).await?;
            run_external_command("chmod", &["755", &user_path]).await?;
            run_external_command("mkdir", &["-p", &user_projects_path]).await?;
            run_external_command("chown", &["root:root", &user_projects_path]).await?;
            run_external_command("chmod", &["755", &user_projects_path]).await?;
        }
        ServerHelperCommand::DeleteUser { user_slug} => {
            run_external_command("userdel", &["--remove", &user_slug]).await?;
        }
        ServerHelperCommand::CreateProject { project_slug: project_slug_str, service_user } => {
            let dev_path = get_project_dev_path(&project_slug_str);
            let prod_path = get_project_prod_path(&project_slug_str);



            // 1. Create the main project Btrfs subvolume
            run_external_command("btrfs", &["subvolume", "create", &dev_path]).await?;
            run_external_command("chown", &["root:root", &dev_path]).await?;
            run_external_command("chmod", &["770", &dev_path]).await?; // Start restrictive

            
            let user_acl_rwx_entry = format!("u:{service_user}:rwX");
            let service_acl_rwx_entry = format!("u:{SERVICE_USER}:rwX");

            run_external_command("setfacl", &["-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-m", &service_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &service_acl_rwx_entry, &dev_path]).await?;


            run_external_command("mkdir", &["-p", &prod_path]).await?;
            run_external_command("chown", &["root:root", &prod_path]).await?;
            run_external_command("chmod", &["755", &prod_path]).await?;

            run_external_command("mkdir", &["-p", &dev_path]).await?;
            run_external_command("chown", &["root:root", &dev_path]).await?;
            run_external_command("chmod", &["770", &dev_path]).await?;
            run_external_command("setfacl", &["-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &user_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-m", &service_acl_rwx_entry, &dev_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &service_acl_rwx_entry, &dev_path]).await?;
        }
        ServerHelperCommand::DeleteProject { project_slug: project_slug_str } => {
            let project_path = get_project_dev_path(&project_slug_str);
            let prod_path = get_project_prod_path(&project_slug_str);
            run_external_command("rm", &["-rf", &prod_path]).await?;
            run_external_command("btrfs", &["subvolume", "delete", &project_path]).await?;
        }
        ServerHelperCommand::SetAcl { path, user_slug, is_read_only } => {
            let perms= if is_read_only {
                "r-X"
            } else {
                "rwX"
            };
            let acl_entry = format!("u:{user_slug}:{perms}");
            run_external_command("setfacl", &["-R", "-m", &acl_entry, &path]).await?;
            run_external_command("setfacl", &["-d", "-m", &acl_entry, &path]).await?;
        }
        ServerHelperCommand::RemoveAcl { path, user_slug } => {
            let acl_spec = format!("u:{user_slug}");
            run_external_command("setfacl", &["-x", &acl_spec, &path]).await?;
            run_external_command("setfacl", &["-d", "-x", &acl_spec, &path]).await?;
        }
        ServerHelperCommand::BindMountUserProject { source_path, target_path } => {
            run_external_command("mkdir", &["-p", &target_path]).await?;
            run_external_command("chown", &["root:root", &target_path]).await?;
            run_external_command("chmod", &["755", &target_path]).await?;
            run_external_command("mount", &["--bind", &source_path, &target_path]).await?;
        }
        ServerHelperCommand::UnmountUserProject { target_path } => {
            // Check if it's actually mounted before trying to unmount?
            // `findmnt -n -o TARGET --target "$target_path"`
            run_external_command("umount", &[&target_path]).await?;
            // Attempt to remove the empty mount point directory after unmounting
            tokio::fs::remove_dir(&target_path).await?;
        }
        ServerHelperCommand::CreateSnapshot { path, snapshot_path } => {
            run_external_command("btrfs", &[
                "subvolume",
                "snapshot",
                "-r", // Read-only flag
                &path,
                &snapshot_path
            ]).await?;
        }
        ServerHelperCommand::DeleteSnapshot {snapshot_path } => {
            run_external_command("btrfs", &["subvolume", "delete", &snapshot_path]).await?;
        }
        ServerHelperCommand::MountSnapshot { path,  snapshot_name } => {
            run_external_command("mount", &["-o",&format!("subvol={snapshot_name},ro"),BTRFS_DEVICE.as_str(), &path]).await?;
        }
        ServerHelperCommand::UnmountProd { path } => {
            run_external_command("umount", &[&path]).await?;
        }
    }
    Ok(())
}

