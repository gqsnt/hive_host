use crate::website_to_server::server_action::ServerAction;
use crate::{impl_chain_from, Slug};

use serde::{Deserialize, Serialize};

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

impl_chain_from!(ServerAction, ServerAction::UserAction => ServerUserAction);
