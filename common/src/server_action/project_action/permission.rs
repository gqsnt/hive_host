use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{impl_chain_from, Slug};

use serde::{Deserialize, Serialize};

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectPermissionAction {
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

impl_chain_from!(ProjectAction, ProjectAction::Permission => ProjectPermissionAction);

impl IsProjectServerAction for ProjectPermissionAction {

    fn permission(&self) -> Permission {
        Permission::Owner
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
