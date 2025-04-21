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
    AddSshKey {
        user_slug: UserSlug,
        ssh_key: String,
    },
    RemoveSshKey {
        user_slug: UserSlug,
        ssh_key: String,
    },
    Delete {
        user_slug: UserSlug,
    },
}

impl_chain_from!(ServerAction, ServerAction::UserAction => UserAction);
