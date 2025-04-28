#![feature(associated_type_defaults)]

pub mod hosting_action;
pub mod permission;
pub mod server_action;
pub mod server_project_action;
pub mod server_helper;

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::marker::PhantomData;
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

pub type ProjectUnixSlugStr = String;
pub type ProjectSlugStr = String;
pub type ProjectSlug = Slug<ProjectId, ProjectSlugStr, ProjectUnixSlugStr>;

pub type UserId = i64;

pub type UserUnixSlugStr = String;
pub type UserSlugStr = String;
pub type UserSlug = Slug<UserId, UserSlugStr, UserUnixSlugStr>;

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Slug<I, U, S> {
    pub id: I,
    pub name: String,
    #[serde(skip)]
    _s: PhantomData<S>,
    #[serde(skip)]
    _u: PhantomData<U>,
}

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum SlugParseError {
    #[error("Missing separator")]
    MissingSeparator,
    #[error("Invalid Id")]
    InvalidId,
    #[error("Empty name")]
    EmptyName,
}


impl<I, S, U> FromStr for Slug<I, S, U>
where
    I: FromStr<Err = ParseIntError>,
    S: From<String>,
    U: From<String>,
{
    type Err = SlugParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((name, id_str)) = s.split_once('-') else {
            return Err(SlugParseError::MissingSeparator);
        };

        let id = id_str.parse::<I>().map_err(|_| SlugParseError::InvalidId)?;

        if name.is_empty() {
            return Err(SlugParseError::EmptyName);
        }

        Ok(Slug {
            id,
            name: name.to_string(),
            _s: PhantomData,
            _u: PhantomData,
        })
    }
}




impl<I, S, U> Slug<I, S, U>
where
    I: Display,
    S: From<String>,
    U: From<String>,
{
    pub fn new(id: I, name: String) -> Self {
        Slug {
            id,
            name,
            _s: PhantomData,
            _u: PhantomData,
        }
    }

    pub fn to_str(&self) -> S {
        S::from(format!("{}-{}", self.name, self.id))
    }

    pub fn to_unix(&self) -> U {
        U::from(format!("{}{}", self.name, self.id).to_lowercase())
    }
}
