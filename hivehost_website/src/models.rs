use common::website_to_server::permission::Permission;
use common::{ProjectId, ProjectSlugStr, Slug, UserId, UserSlugStr};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

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
    pub slug: UserSlugStr,
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

#[derive(Debug, Clone,Default, PartialEq, Eq, Store, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub slug: ProjectSlugStr,
    pub active_snapshot_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub id: i64,
    pub project_id: ProjectId,
    pub name: String,
    pub snapshot_name: String,
    pub description: Option<String>,
    pub created_at: String,
}

impl Project {
    pub fn get_slug(&self) -> Slug {
        Slug::new(self.id, self.name.clone())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SshKeyInfo {
    pub id: i64,
    pub name: String,
    pub user_id: UserId,
}

#[derive(Debug, Clone,Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UserPermission {
    pub user_id: UserId,
    pub project_id: ProjectId,
    pub permission: Permission,
}
