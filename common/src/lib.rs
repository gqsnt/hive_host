#![feature(associated_type_defaults)]

pub mod hosting_action;
pub mod permission;
pub mod server_action;
pub mod server_project_action;
pub mod server_helper;

use serde::{Deserialize, Serialize};
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;

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

pub type ProjectSlugStr = String;

pub type UserId = i64;

pub type UserSlugStr = String;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slug {
    pub id: i64,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ParseSlugError {
    #[error("Invalid slug format")]
    InvalidFormat,
    #[error("Invalid ID in slug")]
    ParseIntError(String),
}

impl From<ParseIntError> for ParseSlugError {
    fn from(err: ParseIntError) -> Self {
        ParseSlugError::ParseIntError(err.to_string())
    }
}


impl Slug {
    pub fn new(id: i64, slug: String) -> Self {
        Slug { id, slug }
    }
}

impl std::fmt::Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.id, self.slug)
    }
}


impl FromStr for Slug {
    type Err = ParseSlugError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('_').collect();
        if parts.len() != 2 {
            return Err(ParseSlugError::InvalidFormat);
        }
        let id = parts[0].parse::<i64>()?;
        let slug = parts[1].to_string();
        Ok(Slug { id, slug })
    }
}




#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StringContent {
    pub inner: Option<String>,
}

impl StringContent {
    pub fn new(content: String) -> Self {
        StringContent {
            inner: Some(content),
        }
    }
}