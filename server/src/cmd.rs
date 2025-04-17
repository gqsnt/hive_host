use std::path::PathBuf;
use common::{ProjectId, ProjectUnixSlugStr, UserId, UserSlugStr, UserUnixSlugStr};
use std::process::Stdio;
use async_tempfile::TempFile;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
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
    let file = OpenOptions::new().read(true).open(path).await?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines();
    let match_line = format!("Match User {}", user_slug);
    let mut inside_block = false;

    // Fichier temporaire
    let mut tmp_file = TempFile::new_in(PathBuf::from("/etc/ssh")).await.unwrap();

    while let Some(line) = lines.next_line().await? {
        if line.trim() == match_line {
            inside_block = true;
            continue; // skip this line
        }

        if inside_block {
            if line.trim().is_empty() {
                inside_block = false;
            }
            continue; // skip lines inside block
        }
        tmp_file.write(line.as_bytes()).await?;
    }

    tmp_file.flush().await?;
    tokio::fs::rename(tmp_file.file_path(), path).await?;
    reload_sshd().await?;
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
