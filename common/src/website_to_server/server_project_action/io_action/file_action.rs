use crate::impl_chain_from;
use crate::website_to_server::permission::Permission;
use crate::website_to_server::server_project_action::io_action::ServerProjectIoAction;
use crate::website_to_server::server_project_action::{IsProjectServerAction, ServerProjectAction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ServerProjectIoFileAction {
    Create { path: String },
    Rename { path: String, new_name: String },
    Delete { path: String },
    Move { path: String, new_path: String },
    Copy { path: String, new_path: String },
    View { path: String },
    Update { path: String },
}

impl_chain_from!(ServerProjectAction, ServerProjectAction::Io | ServerProjectIoAction::File => ServerProjectIoFileAction);

impl IsProjectServerAction for ServerProjectIoFileAction {
    fn with_token(&self) -> bool {
        match self {
            ServerProjectIoFileAction::Rename { .. }
            | ServerProjectIoFileAction::Delete { .. }
            | ServerProjectIoFileAction::Move { .. }
            | ServerProjectIoFileAction::Copy { .. } => false,
            ServerProjectIoFileAction::Create { .. } | ServerProjectIoFileAction::View { .. } | ServerProjectIoFileAction::Update { .. } => true,
        }
    }

    fn permission(&self) -> Permission {
        match self {
            ServerProjectIoFileAction::Create { .. }
            | ServerProjectIoFileAction::Rename { .. }
            | ServerProjectIoFileAction::Delete { .. }
            | ServerProjectIoFileAction::Move { .. }
            | ServerProjectIoFileAction::Copy { .. }
            | ServerProjectIoFileAction::Update { .. } => Permission::Write,
            ServerProjectIoFileAction::View { .. } => Permission::Read,
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ServerProjectIoFileAction::Create { .. }
            | ServerProjectIoFileAction::Rename { .. }
            | ServerProjectIoFileAction::Delete { .. }
            | ServerProjectIoFileAction::Move { .. }
            | ServerProjectIoFileAction::Copy { .. }
            | ServerProjectIoFileAction::Update { .. } => true,
            ServerProjectIoFileAction::View { .. } => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub name: String,
    pub content: String,
    pub path: String,
    pub size: u64,
    pub last_modified: String,
}
