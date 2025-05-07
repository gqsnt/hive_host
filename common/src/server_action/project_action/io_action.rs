use crate::server_action::permission::Permission;
use crate::server_action::project_action::IsProjectServerAction;
use serde::{Deserialize, Serialize};

pub mod dir_action;
pub mod file_action;

#[derive(Debug,Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectIoAction {
    Dir(dir_action::ProjectIoDirAction),
    File(file_action::ProjectIoFileAction),
}

impl IsProjectServerAction for ProjectIoAction {
    fn with_token(&self) -> bool {
        match self {
            ProjectIoAction::Dir(action) => action.with_token(),
            ProjectIoAction::File(action) => action.with_token(),
        }
    }

    fn permission(&self) -> Permission {
        match self {
            ProjectIoAction::Dir(action) => action.permission(),
            ProjectIoAction::File(action) => action.permission(),
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ProjectIoAction::Dir(action) => action.require_csrf(),
            ProjectIoAction::File(action) => action.require_csrf(),
        }
    }
}
