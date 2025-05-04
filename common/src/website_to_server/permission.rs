use bitcode::{Decode, Encode};
use std::fmt::Display;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Decode, Encode, Default, EnumIter)]
#[cfg_attr(feature = "website-ssr", derive(sqlx::Type))]
#[cfg_attr(
    feature = "website-ssr",
    sqlx(type_name = "permission_type", rename_all = "lowercase")
)]
pub enum Permission {
    #[default]
    Read,
    Write,
    Owner,
}

impl Permission {
    pub fn can_edit(&self) -> bool {
        match self {
            Permission::Read => false,
            Permission::Write => true,
            Permission::Owner => true,
        }
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        match self {
            Permission::Read => *permission == Permission::Read,
            Permission::Write => {
                *permission == Permission::Write || *permission == Permission::Read
            }
            Permission::Owner => true,
        }
    }

    pub fn is_owner(&self) -> bool {
        match self {
            Permission::Read => false,
            Permission::Write => false,
            Permission::Owner => true,
        }
    }

    pub fn is_read_only(&self) -> bool {
        match self {
            Permission::Read => true,
            Permission::Write => false,
            Permission::Owner => false,
        }
    }

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
        write!(f, "{str}")
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
