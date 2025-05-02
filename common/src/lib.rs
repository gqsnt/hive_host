#![feature(associated_type_defaults)]


#[cfg(feature = "website-to-hosting")]
pub mod hosting_action;

#[cfg(feature = "website-to-server")]
pub mod permission;


#[cfg(feature = "website-to-server")]
pub mod server_action;


#[cfg(feature = "website-to-server")]
pub mod server_project_action;

#[cfg(feature = "server")]
pub mod server_helper;

#[cfg(feature = "server")]
pub mod command;

use serde::{Deserialize, Serialize};
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;




pub const SERVICE_USER:&str= "hivehost_server";
pub const USER_GROUP:&str= "sftp_users";


pub const DEV_ROOT_PATH_PREFIX: &str = "/hivehost/dev";
pub const PROD_ROOT_PATH_PREFIX: &str = "/hivehost/prod";
pub const USER_ROOT_PATH_PREFIX: &str = "/hivehost/users";


pub fn get_project_dev_path(project_slug_str: &str) -> String {
    format!("{DEV_ROOT_PATH_PREFIX}/{project_slug_str}")
}
pub fn get_project_snapshot_path(snapshot_name: &str) -> String {
    format!("{DEV_ROOT_PATH_PREFIX}/{snapshot_name}")
}



pub fn get_project_prod_path(project_slug_str: &str) -> String {
    format!("{PROD_ROOT_PATH_PREFIX}/{project_slug_str}" )
}



pub fn get_user_path(user_slug_str: &str) -> String {
    format!("{USER_ROOT_PATH_PREFIX}/{user_slug_str}" )
}

pub fn get_user_projects_path(user_slug_str: &str) -> String {
    format!("{}/projects", get_user_path(user_slug_str))
}

pub fn get_user_project_path(user_slug_str: &str, project_slug_str: &str) -> String {
    format!("{}/{}", get_user_projects_path(user_slug_str), project_slug_str)
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
        write!(f, "{}_{}",self.slug, self.id )
    }
}


impl FromStr for Slug {
    type Err = ParseSlugError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.rsplit_once('_'){
            None => {
                Err(ParseSlugError::InvalidFormat)
            }
            Some((name, id)) => {
                if id.is_empty() {
                    return Err(ParseSlugError::InvalidFormat);
                }
                if name.is_empty() {
                    return Err(ParseSlugError::InvalidFormat);
                }
                // check regex for name

                match id.parse::<i64>(){
                    Ok(id) => {
                        if id < 0 {
                            return Err(ParseSlugError::InvalidFormat);
                        }
                        Ok(Slug::new(id, name.to_string()))
                    }
                    Err(e) => {
                         Err(ParseSlugError::ParseIntError(e.to_string()))
                    }
                }
            }
        }
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