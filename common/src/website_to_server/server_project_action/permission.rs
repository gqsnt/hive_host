use crate::website_to_server::permission::Permission;
use crate::website_to_server::server_project_action::{IsProjectServerAction, ServerProjectAction};
use crate::{impl_chain_from, Slug};
use bitcode::{Decode, Encode};

#[derive(Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum ServerProjectPermissionAction {
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

impl_chain_from!(ServerProjectAction, ServerProjectAction::Permission => ServerProjectPermissionAction);

impl IsProjectServerAction for ServerProjectPermissionAction {
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
