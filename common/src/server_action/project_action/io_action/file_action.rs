use crate::impl_chain_from;
use crate::server_action::permission::Permission;
use crate::server_action::project_action::io_action::ProjectIoAction;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};

use serde::{Deserialize, Serialize};

#[derive(Debug,  Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectIoFileAction {
    Create { path: String },
    Rename { path: String, new_name: String },
    Delete { path: String },
    Move { path: String, new_path: String },
    Copy { path: String, new_path: String },
    View { path: String },
    Update { path: String, content: String },
}

impl_chain_from!(ProjectAction, ProjectAction::Io | ProjectIoAction::File => ProjectIoFileAction);

impl IsProjectServerAction for ProjectIoFileAction {
    fn with_token(&self) -> bool {
        false
        // match self {
        //     ServerProjectIoFileAction::Rename { .. }
        //     | ServerProjectIoFileAction::Delete { .. }
        //     | ServerProjectIoFileAction::Move { .. }
        //     | ServerProjectIoFileAction::Copy { .. } => false,
        //     ServerProjectIoFileAction::Create { .. } | ServerProjectIoFileAction::View { .. } | ServerProjectIoFileAction::Update { .. } => true,
        // }
    }

    fn permission(&self) -> Permission {
        match self {
            ProjectIoFileAction::Create { .. }
            | ProjectIoFileAction::Rename { .. }
            | ProjectIoFileAction::Delete { .. }
            | ProjectIoFileAction::Move { .. }
            | ProjectIoFileAction::Copy { .. }
            | ProjectIoFileAction::Update { .. } => Permission::Write,
            ProjectIoFileAction::View { .. } => Permission::Read,
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ProjectIoFileAction::Create { .. }
            | ProjectIoFileAction::Rename { .. }
            | ProjectIoFileAction::Delete { .. }
            | ProjectIoFileAction::Move { .. }
            | ProjectIoFileAction::Copy { .. }
            | ProjectIoFileAction::Update { .. } => true,
            ProjectIoFileAction::View { .. } => false,
        }
    }
}

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub struct FileInfo {
    pub name: String,
    pub content: String,
    pub path: String,
    pub size: u64,
    pub last_modified: String,
}
