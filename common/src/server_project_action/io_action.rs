use serde::{Deserialize, Serialize};
use crate::permission::Permission;
use crate::server_project_action::IsProjectServerAction;

pub mod dir_action;
pub mod file_action;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IoAction {
    Dir(dir_action::DirAction),
    File(file_action::FileAction),
}

impl IsProjectServerAction  for IoAction{
    fn with_token(&self) -> bool {
        match self {
            IoAction::Dir(action) => action.with_token(),
            IoAction::File(action) => action.with_token(),
        }
    }

    fn permission(&self) -> Permission {
        match self {
            IoAction::Dir(action) => action.permission(),
            IoAction::File(action) => action.permission(),
        }
    }
}
