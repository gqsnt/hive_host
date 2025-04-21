#![feature(associated_type_defaults)]

pub mod permission;
pub mod server_action;
pub mod server_project_action;

use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::num::ParseIntError;
use std::str::FromStr;
use serde::de::StdError;
use serde::{Deserialize, Serialize};

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
pub struct StringContent{
    pub inner: Option<String>,
}

impl StringContent{
    pub fn new(content: String) -> Self {
        StringContent {
            inner: Some(content),
        }
    }
}



#[derive(Clone, Debug ,PartialEq, Eq, Serialize, Deserialize)]
pub struct Slug<I, U, S>{
    pub id: I,
    pub name: String,
    #[serde(skip)]
    _s: PhantomData<S>,
    #[serde(skip)]
    _u: PhantomData<U>,
}

#[derive(Debug)]
pub enum SlugParseError {
    MissingSeparator,
    InvalidId(ParseIntError),
    EmptyName,
}

impl Display for SlugParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SlugParseError::MissingSeparator => write!(f, "Missing separator '-' in project slug"),
            SlugParseError::InvalidId(err) => write!(f, "Invalid project id: {}", err),
            SlugParseError::EmptyName => write!(f, "Project name is empty"),
        }
    }
}

impl StdError for SlugParseError {}

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

        let id = id_str.parse::<I>()
            .map_err(SlugParseError::InvalidId)?;

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
        U::from(format!("{}{}", self.name, self.id))
    }
}




