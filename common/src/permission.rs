


use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature="ssr", derive(sqlx::Type))]
pub enum Permission {
    Read,
    Write,
    Owner,
}
