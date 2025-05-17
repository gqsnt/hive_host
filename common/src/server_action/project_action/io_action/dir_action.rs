use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{Validate, impl_chain_from};

use crate::server_action::project_action::io_action::ProjectIoAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectIoDirAction {
    Create { path: String },
    Rename { path: String, new_name: String },
    Delete { path: String },
    Ls { path: String },
}

impl_chain_from!(ProjectAction , ProjectAction::Io | ProjectIoAction::Dir  => ProjectIoDirAction);

impl Validate for ProjectIoDirAction {
    fn validate(&self) -> Result<(), crate::SanitizeError> {
        match self {
            ProjectIoDirAction::Create { path: _ } => {}
            ProjectIoDirAction::Rename {
                path: _,
                new_name: _,
            } => {}
            ProjectIoDirAction::Delete { path: _ } => {}
            ProjectIoDirAction::Ls { path: _ } => {}
        }
        Ok(())
    }
}

impl IsProjectServerAction for ProjectIoDirAction {
    fn permission(&self) -> Permission {
        match self {
            ProjectIoDirAction::Create { .. }
            | ProjectIoDirAction::Rename { .. }
            | ProjectIoDirAction::Delete { .. } => Permission::Write,
            ProjectIoDirAction::Ls { .. } => Permission::Read,
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ProjectIoDirAction::Create { .. }
            | ProjectIoDirAction::Rename { .. }
            | ProjectIoDirAction::Delete { .. } => true,
            ProjectIoDirAction::Ls { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ServerProjectIoDirActionLsResponse {
    pub inner: Vec<LsElement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LsElement {
    pub name: String,
    pub is_dir: bool,
}
