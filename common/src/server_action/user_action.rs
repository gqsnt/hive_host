use crate::server_action::ServerAction;
use crate::{impl_chain_from, Slug};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UserAction {
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

impl_chain_from!(ServerAction, ServerAction::UserAction => UserAction);
