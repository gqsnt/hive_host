#![feature(associated_type_defaults)]

#[cfg(feature = "server-command")]
pub mod command;

#[cfg(feature = "website-to-server")]
pub mod server_action;

#[cfg(feature = "hosting")]
pub mod hosting_command;

#[cfg(feature = "tarpc-client")]
pub mod tarpc_client;

#[cfg(feature = "server-to-helper")]
pub mod helper_command;

use reactive_stores::Patch;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

pub const SERVICE_USER: &str = "hivehost_server";
pub const USER_GROUP: &str = "sftp_users";
pub const SERVER_PORT: u16 = 5051;
pub const SERVER_TOKEN_PORT: u16 = 5052;
pub const GITHUB_APP_NAME: &str = "hivehost git";

pub const DEV_ROOT_PATH_PREFIX: &str = "/hivehost/dev";
pub const PROD_ROOT_PATH_PREFIX: &str = "/hivehost/prod";
pub const USER_ROOT_PATH_PREFIX: &str = "/hivehost/users";
pub const TEMP_ROOT_PATH_PREFIX: &str = "/hivehost/temp";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum AuthResponse {
    Ok,
    Error,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AuthToken(pub String);
impl FromStr for AuthToken {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.len() != 40 || !s.chars().all(|c| c.is_alphanumeric()) {
            return sanitize_err();
        }
        Ok(AuthToken(s.to_string()))
    }
}
impl std::fmt::Display for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "autth_token")
    }
}
impl std::fmt::Debug for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "auth_token")
    }
}

impl Validate for AuthToken {
    fn validate(&self) -> Result<(), SanitizeError> {
        Self::from_str(&self.0)?;
        Ok(())
    }
}

pub fn get_temp_token_path(token: &str) -> String {
    format!("{TEMP_ROOT_PATH_PREFIX}/{token}")
}

pub fn get_project_dev_path(project_slug_str: &ProjectSlugStr) -> String {
    format!("{DEV_ROOT_PATH_PREFIX}/{}", project_slug_str.0)
}
pub fn get_project_snapshot_path(snapshot_name: &str) -> String {
    format!("{DEV_ROOT_PATH_PREFIX}/{snapshot_name}")
}

pub fn get_project_prod_path(project_slug_str: &ProjectSlugStr) -> String {
    format!("{PROD_ROOT_PATH_PREFIX}/{}", project_slug_str.0)
}

pub fn get_user_path(user_slug_str: &UserSlugStr) -> String {
    format!("{USER_ROOT_PATH_PREFIX}/{}", user_slug_str.0)
}

pub fn get_user_projects_path(user_slug_str: &UserSlugStr) -> String {
    format!("{}/projects", get_user_path(user_slug_str))
}

pub fn get_user_project_path(
    user_slug_str: &UserSlugStr,
    project_slug_str: &ProjectSlugStr,
) -> String {
    format!(
        "{}/{}",
        get_user_projects_path(user_slug_str),
        project_slug_str.0
    )
}

#[macro_export]
macro_rules! impl_chain_from {
    ($target_type:path, $($chain:path)|+ => $source:ty $(,)?) => {
        impl From<$source> for $target_type {
            fn from(value: $source) -> Self {
                impl_chain_from!(@wrap value, $($chain)|+)
            }
        }
    };

    (@wrap $val:ident, $head:path | $($rest:path)|+ ) => {
        $head(impl_chain_from!(@wrap $val, $($rest)|+))
    };

    (@wrap $val:ident, $last:path) => {
        $last($val)
    };
}

pub type ProjectId = i64;
pub type ServerId = i64;
pub type UserId = i64;

pub trait Validate {
    fn validate(&self) -> Result<(), SanitizeError>;
}

#[cfg(feature = "validate-path")]
pub async fn ensure_path_in_project_path(
    project_slug: &ProjectSlugStr,
    project_path_: &str,
    is_file: bool,
    should_exist: bool,
) -> Result<std::path::PathBuf, SanitizeError> {
    // 1) Canonicaliser la racine projet
    let mut project_path_ = project_path_.to_string();
    if !project_path_.starts_with("root/") {
        return sanitize_err();
    }
    project_path_ = project_path_.replacen("root/", "./", 1);

    let project_root = std::path::PathBuf::from(get_project_dev_path(project_slug));
    let project_root = tokio::fs::canonicalize(&project_root).await?;

    // 2) Rejeter tout chemin absolu ou contenant `..`
    let rel = std::path::PathBuf::from(project_path_);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return sanitize_err();
    }

    // Chemin final (peut ne pas exister)
    let full_path = project_root.join(rel);

    if should_exist {
        // 3A) On attend que la cible existe → canonicaliser puis métadonnées
        let canon = tokio::fs::canonicalize(&full_path).await?;

        // 4A) Vérifier qu’elle reste sous project_root
        if !canon.starts_with(&project_root) {
            return Err(SanitizeError::Invalid);
        }

        // 5A) Vérifier fichier vs dossier
        let meta = tokio::fs::metadata(&canon).await?;
        if is_file && !meta.is_file() {
            return Err(SanitizeError::Invalid);
        }
        if !is_file && !meta.is_dir() {
            return Err(SanitizeError::Invalid);
        }

        Ok(canon)
    } else {
        // 3B) Création de la cible → vérifier uniquement le parent
        let parent = full_path.parent().ok_or(SanitizeError::Invalid)?;
        let parent_canon = tokio::fs::canonicalize(parent).await?;

        // 4B) S’assurer que le parent est dans le projet
        if !parent_canon.starts_with(&project_root) {
            return sanitize_err();
        }
        // 5B) Vérifier que le parent est un dossier
        let meta = tokio::fs::metadata(&parent_canon).await?;
        if !meta.is_dir() {
            return Err(SanitizeError::Invalid);
        }
        // 6B) Vérifier que le nom est sanitisé
        let child = full_path.file_name().ok_or(SanitizeError::Invalid)?;
        let sanitized = sanitize_filename::sanitize(child.to_str().unwrap_or_default());
        if sanitized.is_empty() {
            return Err(SanitizeError::Invalid);
        }

        // 7B) OK pour créer : retourner le chemin (non-canon) où l’on créera.
        Ok(parent_canon.join(sanitized))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct SnapShotNameStr(pub String);

impl Validate for SnapShotNameStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        Self::from_str(&self.0)?;
        Ok(())
    }
}

// validated project slug
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct ProjectSlugStr(pub String);
impl Validate for ProjectSlugStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        Slug::from_str(&self.0)?;
        Ok(())
    }
}

// validated user slug
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct UserSlugStr(pub String);
impl Validate for UserSlugStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        Slug::from_str(&self.0)?;
        Ok(())
    }
}

// validated branch name
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct GitBranchNameStr(pub String);
impl Validate for GitBranchNameStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        Self::from_str(&self.0)?;
        Ok(())
    }
}

// validated git full name
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct GitRepoFullNameStr(pub String);
impl Validate for GitRepoFullNameStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        Self::from_str(&self.0)?;
        Ok(())
    }
}

// validated git commit
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct GitCommitStr(pub String);
impl Validate for GitCommitStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        Self::from_str(&self.0)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct GitTokenStr(pub String);
impl Validate for GitTokenStr {
    fn validate(&self) -> Result<(), SanitizeError> {
        GitTokenStr::from_str(&self.0)?;
        Ok(())
    }
}

impl std::fmt::Display for GitTokenStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "secret_token")
    }
}

impl std::fmt::Debug for GitTokenStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "secret_token")
    }
}

impl FromStr for GitTokenStr {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.len() > 40 || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return sanitize_err();
        }
        Ok(GitTokenStr(s.to_string()))
    }
}

impl FromStr for GitCommitStr {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.len() > 40 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return sanitize_err();
        }
        Ok(GitCommitStr(s.to_string()))
    }
}

impl FromStr for SnapShotNameStr {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty()
            || s.len() > 40
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return sanitize_err();
        }
        Ok(SnapShotNameStr(s.to_string()))
    }
}

impl FromStr for GitRepoFullNameStr {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return sanitize_err();
        }
        let (username, repo_name) = s.split_once('/').ok_or_else(SanitizeError::default)?;
        if username.is_empty()
            || repo_name.is_empty()
            || username.len() > 39
            || !username
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                | !repo_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return sanitize_err();
        }
        Ok(GitRepoFullNameStr(s.to_string()))
    }
}

impl FromStr for GitBranchNameStr {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty()
            || s.starts_with('.')
            || s.ends_with('/')
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
        {
            return sanitize_err();
        }
        Ok(GitBranchNameStr(s.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Patch)]
pub struct Slug {
    pub id: i64,
    pub slug: String,
}

impl Slug {
    pub fn to_project_slug_str(&self) -> ProjectSlugStr {
        ProjectSlugStr(self.to_string())
    }

    pub fn to_user_slug_str(&self) -> UserSlugStr {
        UserSlugStr(self.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Deserialize, Serialize, Default)]
pub enum SanitizeError {
    #[default]
    #[error("Sanitization check failed")]
    Invalid,
}

#[cfg(feature = "validate-path")]
impl From<std::io::Error> for SanitizeError {
    fn from(_err: std::io::Error) -> Self {
        SanitizeError::Invalid
    }
}

pub fn sanitize_err<T>() -> Result<T, SanitizeError> {
    Err(SanitizeError::Invalid)
}

impl Slug {
    pub fn new(id: i64, slug: String) -> Self {
        Slug { id, slug }
    }
}

impl std::fmt::Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.slug, self.id)
    }
}

impl FromStr for Slug {
    type Err = SanitizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.rsplit_once('-') {
            None => Err(SanitizeError::default()),
            Some((name, id)) => {
                if id.is_empty() {
                    return sanitize_err();
                }
                if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return sanitize_err();
                }

                match id.parse::<i64>() {
                    Ok(id) => {
                        if id < 0 {
                            return sanitize_err();
                        }
                        Ok(Slug::new(id, name.to_string()))
                    }
                    Err(_) => sanitize_err(),
                }
            }
        }
    }
}
