use common::{ProjectId, ProjectUnixSlugStr, UserId, UserUnixSlugStr};
use std::process::Stdio;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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

pub async fn append_ssh_match_user(
    file_append: &mut tokio::fs::File,
    user_slug:UserUnixSlugStr,
) -> Result<(), tokio::io::Error> {
    // Blocs multi-lignes
    let block = format!(
        r#"

Match User {user_slug}
  ChrootDirectory /sftp/users/{user_slug}
  ForceCommand internal-sftp
  AllowTCPForwarding no
  X11Forwarding no

"#
    );
    file_append.write(block.as_bytes()).await?;
    Ok(())
}

pub async fn reload_sshd() -> Result<(), tokio::io::Error> {
    run_sudo_cmd(&["systemctl", "reload", "sshd"]).await
}

pub async fn ensure_sshd_match_user(user_slug:UserUnixSlugStr) -> Result<(), tokio::io::Error> {
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
    append_ssh_match_user(&mut file_append, user_slug).await?;
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
