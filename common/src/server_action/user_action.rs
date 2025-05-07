use crate::{Slug};

use serde::{Deserialize, Serialize};
use crate::helper_command::HelperResponse;

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ServerUserAction {
    Create {
        user_slug: Slug,
    },
    AddProject {
        user_slug: Slug,
        project_slug: Slug,
    },
    RemoveProject {
        user_slugs: Vec<Slug>,
        project_slug: Slug,
    },
    Delete {
        user_slug: Slug,
    },
}


#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ServerUserResponse{
    Ok,
    Helper(HelperResponse),
    Error(String),
}