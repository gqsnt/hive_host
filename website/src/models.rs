use common::permission::Permission;
use common::{ProjectId, ProjectSlug, Slug, UserId, UserSlug};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Default)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub role_type: RoleType,
    pub username: String,
}

impl User {
    pub fn get_slug(&self) -> UserSlug {
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
            email: "guest@mail.com".to_string(),
            role_type: RoleType::default(),
            username: "guest".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub is_active: bool,
}

impl Project {
    pub fn get_slug(&self) -> ProjectSlug {
        ProjectSlug::new(self.id, self.name.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SshKeyInfo {
    pub id: i64,
    pub name: String,
    pub user_id: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UserPermission {
    pub user_id: UserId,
    pub project_id: ProjectId,
    pub permission: Permission,
}
