use crate::server_action::ServerAction;
use crate::{impl_chain_from, ProjectSlug, UserSlug};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UserAction {
    Create {
        user_slug: UserSlug,
    },
    AddProject {
        user_slug: UserSlug,
        project_slug: ProjectSlug,
    },
    RemoveProject {
        user_slugs: Vec<UserSlug>,
        project_slug: ProjectSlug,
    },
    Delete {
        user_slug: UserSlug,
    },
}

impl_chain_from!(ServerAction, ServerAction::UserAction => UserAction);
