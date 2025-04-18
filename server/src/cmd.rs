use std::path::PathBuf;
use common::{ProjectId, ProjectUnixSlugStr, UserId, UserSlugStr, UserUnixSlugStr};
use std::process::Stdio;
use async_tempfile::TempFile;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;
use crate::{ServerProjectId, ServerUserId};

pub async fn run_sudo_cmd(args: &[&str]) -> Result<(), tokio::io::Error> {
    let status = Command::new("sudo")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;
    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Command failed: {:?}", args),
        ));
    }
    Ok(())
}


pub async fn set_acl(path: &str, user_slug:UserUnixSlugStr, perms: &str) -> Result<(), tokio::io::Error> {
    run_sudo_cmd(
        &["setfacl","-m", &format!("u:{}:{}", user_slug, perms), path],
    )
    .await?;
    run_sudo_cmd(
        &["setfacl", "-d", "-m", &format!("u:{}:{}", user_slug, perms), path],
    )
    .await?;
    Ok(())
}


pub async fn reload_sshd() -> Result<(), tokio::io::Error> {
    run_sudo_cmd(&["systemctl", "reload", "sshd"]).await
}

pub fn ssh_config_block(user_slug:UserUnixSlugStr) -> String{
    format!(
        r#"

Match User {user_slug}
  ChrootDirectory /sftp/users/{user_slug}
  ForceCommand internal-sftp
  AllowTCPForwarding no
  X11Forwarding no

"#
    )
}


pub async fn ensure_user_in_sshd(user_slug:UserUnixSlugStr) -> Result<(), tokio::io::Error> {
    let path = "/etc/ssh/sshd_config";
    let file = OpenOptions::new().read(true).open(path).await?;
    let mut lines = BufReader::new(file).lines();

    let match_line = format!("Match User {}", user_slug);
    while let Some(line) = lines.next_line().await? {
        if line.trim() == match_line {
            return Ok(());
        }
    }
    let mut file_append = OpenOptions::new().append(true).open(path).await?;
    let block = ssh_config_block(user_slug);
    file_append.write(block.as_bytes()).await?;
    reload_sshd().await?;
    Ok(())
}

pub async fn ensure_user_removed_in_sshd(user_slug: UserSlugStr) -> Result<(), tokio::io::Error> {
    let path = "/etc/ssh/sshd_config";
    remove_block(path, &ssh_config_block(user_slug)).await?;
    reload_sshd().await?;
    Ok(())
}

pub async fn remove_block<P>(file_path: P, block: &str) -> tokio::io::Result<()>
where
    P: AsRef<std::path::Path>,
{
    let path = file_path.as_ref();

    // Nothing to do if the provided block string is empty or whitespace­-only
    if block.trim().is_empty() {
        return Ok(());
    }

    // Identify the start and end markers in the block
    let mut block_lines = block.lines();
    let start_marker = block_lines
        .next()
        .ok_or_else(|| tokio::io::Error::new(tokio::io::ErrorKind::InvalidInput, "Empty removal block"))?;
    let end_marker = block_lines.last().unwrap_or(start_marker);

    // Open source file for reading
    let src = tokio::fs::File::open(path).await?;
    let mut reader = BufReader::new(src).lines();

    // Prepare a temporary file alongside the original
    let mut tmp = TempFile::new_with_uuid_in(
        Uuid::new_v4(),
        path.parent().unwrap()
    ).await.unwrap();

    // Walk through each line, skipping the block once it's found
    let mut skipping = false;
    while let Some(line) = reader.next_line().await? {
        if !skipping && line.contains(start_marker) {
            // We've hit the start of the block: begin skipping
            skipping = true;
            continue;
        }

        if skipping {
            // We're in the block: check for its end
            if line.contains(end_marker) {
                // End of block reached: resume writing subsequent lines
                skipping = false;
            }
            continue;
        }

        // Not in skip-mode: write the line back
        tmp.write_all(line.as_bytes()).await?;
        tmp.write_all(b"\n").await?;
    }

    // Finalize and atomically replace original
    tmp.flush().await?;
    tokio::fs::rename(&tmp.file_path(), path).await?;
    Ok(())
}




pub async fn bind_project_to_user_chroot(
    user_slug:UserUnixSlugStr,
    project_slug: ProjectUnixSlugStr,
) -> Result<(), tokio::io::Error> {
    let user_mount_point = user_project_path(user_slug, project_slug.clone());
    tokio::fs::create_dir_all(&user_mount_point).await?;
    run_sudo_cmd(
        &["mount", "--bind", &project_path(project_slug), &user_mount_point],
    )
    .await?;
    Ok(())
}

pub fn project_path(project_slug: ProjectUnixSlugStr) -> String {
    format!("/projects/{}", project_slug)
}

pub fn user_project_path(user_slug:UserUnixSlugStr, project_slug: ProjectUnixSlugStr) -> String {
    format!("{}/{}", user_projects_path(user_slug), project_slug)
}
pub fn user_projects_path(user_slug:UserUnixSlugStr) -> String {
    format!("{}/projects", user_path(user_slug))
}

pub fn user_path(user_slug:UserUnixSlugStr) -> String {
    format!("/sftp/users/{}", user_slug)
}

pub fn ssh_path(user_slug:UserUnixSlugStr) -> String {
    format!("{}/.ssh", user_path(user_slug))
}

pub fn ssh_key_path(user_slug:UserUnixSlugStr) -> String {
    format!("{}/authorized_keys", ssh_path(user_slug))
}
