use common::server_action::permission::Permission;
use common::{ProjectId, ServerId, Slug, UserId};
use reactive_stores::{Patch, Store};
use serde::{Deserialize, Serialize};

pub type UserSlugStrFront = String;
pub type ProjectSlugStrFront = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[cfg_attr(
    feature = "ssr",
    sqlx(type_name = "role_type", rename_all = "lowercase")
)]
pub enum RoleType {
    #[default]
    User,
    Admin,
}

#[derive(Clone, Debug, Store, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub role_type: RoleType,
    pub username: String,
    pub slug: UserSlugStrFront,
}

impl User {
    pub fn get_slug(&self) -> Slug {
        Slug::new(self.id, self.username.clone())
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for User {}

impl Default for User {
    fn default() -> Self {
        Self {
            id: -1,
            role_type: RoleType::default(),
            username: "guest".to_string(),
            slug: "guest".to_string(),
        }
    }
}

#[derive(Store, Patch, Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Project {
    pub id: ProjectId,
    pub server_id: ServerId,
    pub hosting_address: String,
    pub name: String,
    pub slug: ProjectSlugStrFront,
    pub active_snapshot_id: Option<i64>,
    pub git_project: Option<GitProject>,
}

#[derive(Store, Patch, Debug, Clone, Serialize, Deserialize)]
pub struct GitProject {
    pub id: i64,
    pub repo_full_name: String,
    pub branch_name: String,
    pub dev_commit: String,
    pub prod_branch_commit: Option<(String, String)>, // (branch, commit)
    pub last_commit: String,
    pub auto_deploy: bool,
    pub installation_id: i64,
    pub user_githubs_id: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Store, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Server {
    pub id: ServerId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub id: i64,
    pub project_id: ProjectId,
    pub name: String,
    pub version: i64,
    pub snapshot_name: String,
    pub description: Option<String>,
    pub git_commit: Option<String>,
    pub git_branch: Option<String>,
    pub created_at: String,
}

impl Project {
    pub fn get_slug(&self) -> Slug {
        Slug::new(self.id, self.name.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UserGithub {
    pub id: i64,
    pub login: String,
    pub installation_id: i64,
    pub avatar_url: String,
    pub html_url: String,
    pub suspended: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SshKeyInfo {
    pub id: i64,
    pub name: String,
    pub user_id: UserId,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UserPermission {
    pub user_id: UserId,
    pub project_id: ProjectId,
    pub permission: Permission,
}
