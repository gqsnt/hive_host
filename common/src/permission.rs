use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum_macros::EnumIter;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Default, EnumIter,
)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[cfg_attr(
    feature = "ssr",
    sqlx(type_name = "permission_type", rename_all = "lowercase")
)]
pub enum Permission {
    #[default]
    Read,
    Write,
    Owner,
}

impl Permission {
    pub fn label(&self) -> &'static str {
        match self {
            Permission::Read => "Read",
            Permission::Write => "Write",
            Permission::Owner => "Owner",
        }
    }

    pub fn acl(&self) -> &'static str {
        match self {
            Permission::Read => "r",
            Permission::Write => "w",
            Permission::Owner => "o",
        }
    }
}

impl Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Permission::Read => "Read".to_string(),
            Permission::Write => "Write".to_string(),
            Permission::Owner => "Owner".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl From<&str> for Permission {
    fn from(path: &str) -> Self {
        match path {
            "read" => Permission::Read,
            "write" => Permission::Write,
            "owner" => Permission::Owner,
            _ => Permission::default(),
        }
    }
}
