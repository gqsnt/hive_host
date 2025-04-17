use crate::permission::Permission;
use serde::{Deserialize, Serialize};
use crate::server_project_action::{IsProjectServerAction, ServerProjectAction};
use crate::{impl_chain_from, UserSlug};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PermissionAction{
    Grant{
        user_slug:UserSlug,
        permission:Permission,
    },
    Revoke{
        user_slug:UserSlug,
        permission:Permission,
    },
    Change{
        user_slug:UserSlug,
        permission:Permission,
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
}
