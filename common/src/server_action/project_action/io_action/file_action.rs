use crate::server_action::permission::Permission;
use crate::server_action::project_action::io_action::ProjectIoAction;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{Validate, impl_chain_from};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectIoFileAction {
    Create { path: String },
    Rename { path: String, new_name: String },
    Delete { path: String },
    Move { path: String, new_path: String },
    Copy { path: String, new_path: String },
}

impl_chain_from!(ProjectAction, ProjectAction::Io | ProjectIoAction::File => ProjectIoFileAction);

impl Validate for ProjectIoFileAction {
    fn validate(&self) -> Result<(), crate::SanitizeError> {
        match self {
            ProjectIoFileAction::Create { path: _ } => {}
            ProjectIoFileAction::Rename {
                path: _,
                new_name: _,
            } => {}
            ProjectIoFileAction::Delete { path: _ } => {}
            ProjectIoFileAction::Move {
                path: _,
                new_path: _,
            } => {}
            ProjectIoFileAction::Copy {
                path: _,
                new_path: _,
            } => {}
        }
        Ok(())
    }
}

impl IsProjectServerAction for ProjectIoFileAction {
    fn permission(&self) -> Permission {
        Permission::Write
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
