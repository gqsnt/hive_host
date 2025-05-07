use crate::impl_chain_from;
use crate::server_action::permission::Permission;
use crate::server_action::project_action::io_action::ProjectIoAction;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};

use serde::{Deserialize, Serialize};

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectIoDirAction {
    Create { path: String },
    Rename { path: String, new_name: String },
    Delete { path: String },
    Ls { path: String },
    Download,
}

impl_chain_from!(ProjectAction , ProjectAction::Io | ProjectIoAction::Dir  => ProjectIoDirAction);

impl IsProjectServerAction for ProjectIoDirAction {
    fn with_token(&self) -> bool {
        false
        // match self {
        //     ServerProjectIoDirAction::Create { .. }
        //     | ServerProjectIoDirAction::Rename { .. }
        //     | ServerProjectIoDirAction::Delete { .. }
        //     | ServerProjectIoDirAction::Ls { .. } => false,
        //     ServerProjectIoDirAction::Download => true,
        // }
    }

    fn permission(&self) -> Permission {
        match self {
            ProjectIoDirAction::Create { .. } | ProjectIoDirAction::Rename { .. } | ProjectIoDirAction::Delete { .. } => {
                Permission::Write
            }
            ProjectIoDirAction::Download | ProjectIoDirAction::Ls { .. } => Permission::Read,
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ProjectIoDirAction::Create { .. } | ProjectIoDirAction::Rename { .. } | ProjectIoDirAction::Delete { .. } => true,
            ProjectIoDirAction::Download | ProjectIoDirAction::Ls { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub struct ServerProjectIoDirActionLsResponse {
    pub inner: Vec<LsElement>,
}

#[derive(Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub struct LsElement {
    pub name: String,
    pub is_dir: bool,
}
