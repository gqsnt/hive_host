use crate::permission::Permission;
use crate::server_project_action::{IsProjectServerAction, ServerProjectAction};
use crate::{impl_chain_from, Slug};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PermissionAction {
    Grant {
        user_slug: Slug,
        permission: Permission,
    },
    Revoke {
        user_slug: Slug,
    },
    Update {
        user_slug: Slug,
        permission: Permission,
    },
}

impl_chain_from!(ServerProjectAction, ServerProjectAction::Permission => PermissionAction);

impl IsProjectServerAction for PermissionAction {
    fn with_token(&self) -> bool {
        false
    }

    fn permission(&self) -> Permission {
        Permission::Owner
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
