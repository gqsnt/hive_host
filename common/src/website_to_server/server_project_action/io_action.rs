use crate::website_to_server::permission::Permission;
use crate::website_to_server::server_project_action::IsProjectServerAction;
use serde::{Deserialize, Serialize};

pub mod dir_action;
pub mod file_action;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ServerProjectIoAction {
    Dir(dir_action::ServerProjectIoDirAction),
    File(file_action::ServerProjectIoFileAction),
}

impl IsProjectServerAction for ServerProjectIoAction {
    fn with_token(&self) -> bool {
        match self {
            ServerProjectIoAction::Dir(action) => action.with_token(),
            ServerProjectIoAction::File(action) => action.with_token(),
        }
    }

    fn permission(&self) -> Permission {
        match self {
            ServerProjectIoAction::Dir(action) => action.permission(),
            ServerProjectIoAction::File(action) => action.permission(),
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ServerProjectIoAction::Dir(action) => action.require_csrf(),
            ServerProjectIoAction::File(action) => action.require_csrf(),
        }
    }
}
