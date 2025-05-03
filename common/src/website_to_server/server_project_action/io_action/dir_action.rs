use crate::impl_chain_from;
use crate::website_to_server::permission::Permission;
use crate::website_to_server::server_project_action::io_action::ServerProjectIoAction;
use crate::website_to_server::server_project_action::{IsProjectServerAction, ServerProjectAction};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerProjectIoDirAction {
    Create { path: String },
    Rename { path: String, new_name: String },
    Delete { path: String },
    Ls { path: String },
    Download,
}

impl_chain_from!(ServerProjectAction , ServerProjectAction::Io | ServerProjectIoAction::Dir  => ServerProjectIoDirAction);

impl IsProjectServerAction for ServerProjectIoDirAction {
    fn with_token(&self) -> bool {
        match self {
            ServerProjectIoDirAction::Create { .. }
            | ServerProjectIoDirAction::Rename { .. }
            | ServerProjectIoDirAction::Delete { .. }
            | ServerProjectIoDirAction::Ls { .. } => false,
            ServerProjectIoDirAction::Download => true,
        }
    }

    fn permission(&self) -> Permission {
        match self {
            ServerProjectIoDirAction::Create { .. } | ServerProjectIoDirAction::Rename { .. } | ServerProjectIoDirAction::Delete { .. } => {
                Permission::Write
            }
            ServerProjectIoDirAction::Download | ServerProjectIoDirAction::Ls { .. } => Permission::Read,
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ServerProjectIoDirAction::Create { .. } | ServerProjectIoDirAction::Rename { .. } | ServerProjectIoDirAction::Delete { .. } => true,
            ServerProjectIoDirAction::Download | ServerProjectIoDirAction::Ls { .. } => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ServerProjectIoDirActionLsResponse {
    pub inner: Vec<LsElement>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LsElement {
    pub name: String,
    pub is_dir: bool,
}
