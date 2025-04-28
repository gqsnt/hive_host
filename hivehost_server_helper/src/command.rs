use std::process::Stdio;
use tokio::process::Command as TokioCommand;
use tracing::{error, info, instrument, warn};
use common::server_helper::ServerHelperCommand;
use crate::{ServerHelperError, ServerHelperResult, USER_GROUP};


#[instrument(skip(command), fields(command_type = std::any::type_name::<ServerHelperCommand>()))]
pub async fn execute_command(command: ServerHelperCommand) -> ServerHelperResult<()> {
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
            //    Ownership root:root, mode 755 is fine for this mount parent
            run_external_command("chown", &["root:root", &user_projects_path]).await?;
            run_external_command("chmod", &["755", &user_projects_path]).await?;
        }
        ServerHelperCommand::DeleteUser { user_slug} => {
            run_external_command("userdel", &["--remove", &user_slug]).await?;
        }
        ServerHelperCommand::CreateProjectDir { project_path, service_user } => {
            // 1. Create the directory
            run_external_command("mkdir", &["-p", &project_path]).await?;
            run_external_command("chown", &["root:root", &project_path]).await?;
            run_external_command("chmod", &["700", &project_path]).await?; // Start restrictive

            // 2. Grant service user access via ACL
            let acl_entry = format!("u:{}:rwx", service_user);
            run_external_command("setfacl", &["-m", &acl_entry, &project_path]).await?;
            run_external_command("setfacl", &["-d", "-m", &acl_entry, &project_path]).await?;
        }
        ServerHelperCommand::DeleteProjectDir { project_path } => {
            run_external_command("rm", &["-rf", &project_path]).await?;
        }
        ServerHelperCommand::SetAcl { path, user_slug, is_read_only } => {
            let perms= if is_read_only {
                "r-X"
            } else {
                "rwX"
            };
            let acl_entry = format!("u:{}:{}", user_slug, perms);
            run_external_command("setfacl", &["-m", &acl_entry, &path]).await?;
            run_external_command("setfacl", &["-d", "-m", &acl_entry, &path]).await?;
        }
        ServerHelperCommand::RemoveAcl { path, user_slug } => {
            let acl_spec = format!("u:{}", user_slug);
            run_external_command("setfacl", &["-x", &acl_spec, &path]).await?;
            run_external_command("setfacl", &["-d", "-x", &acl_spec, &path]).await?;
        }
        ServerHelperCommand::BindMount { source_path, target_path } => {
            run_external_command("mkdir", &["-p", &target_path]).await?;
            run_external_command("mount", &["--bind", &source_path, &target_path]).await?;
        }
        ServerHelperCommand::Unmount { target_path } => {
            // Check if it's actually mounted before trying to unmount?
            // `findmnt -n -o TARGET --target "$target_path"`
            match run_external_command("umount", &[&target_path]).await {
                Ok(_) => info!("Unmounted {}", target_path),
                Err(e) => {
                    // umount often fails if not mounted, which might be ok
                    warn!("Failed to unmount {} (maybe not mounted?): {}", target_path, e);
                    // Decide if this should be a hard error or just a warning
                    // For idempotency, maybe just warn.
                    // return Err(anyhow!("Failed to unmount: {}", e));
                }
            };
            // Attempt to remove the empty mount point directory after unmounting
            if let Err(e) = tokio::fs::remove_dir(&target_path).await {
                warn!("Failed to remove mount point dir {}: {}", target_path, e);
            }
        }
    }
    Ok(())
}

#[instrument]
async fn run_external_command(program: &str, args: &[&str]) -> ServerHelperResult<String> {
    info!("Running: {} {}", program, args.join(" "));
    let output = TokioCommand::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output() // Use output() to get status and stdio
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        error!(
            "Command '{} {}' failed with status: {}. Stderr: {}",
            program, args.join(" "), output.status, stderr.trim()
        );
        return Err(ServerHelperError::Other(format!(
            "Command failed: {}. Stderr: {}",
            program, stderr.trim()
        )));
    }

    info!("Command successful: {} {}", program, args.join(" "));
    Ok(stdout) // Return stdout if needed, otherwise just Ok(())
}
