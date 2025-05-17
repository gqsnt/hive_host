use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{UserSlugStr, Validate, impl_chain_from};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectPermissionAction {
    Grant {
        user_slug: UserSlugStr,
        permission: Permission,
    },
    Revoke {
        user_slug: UserSlugStr,
    },
    Update {
        user_slug: UserSlugStr,
        permission: Permission,
    },
}

impl Validate for ProjectPermissionAction {
    fn validate(&self) -> Result<(), crate::SanitizeError> {
        match self {
            ProjectPermissionAction::Grant {
                user_slug,
                permission: _,
            } => {
                user_slug.validate()?;
            }
            ProjectPermissionAction::Revoke { user_slug } => {
                user_slug.validate()?;
            }
            ProjectPermissionAction::Update {
                user_slug,
                permission: _,
            } => {
                user_slug.validate()?;
            }
        }
        Ok(())
    }
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
