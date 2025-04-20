use crate::permission::Permission;
use crate::server_project_action::{IsProjectServerAction, ServerProjectAction};
use serde::{Deserialize, Serialize};
use crate::impl_chain_from;
use crate::server_project_action::io_action::IoAction;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FileAction {
    Create {
        path: String,
        name: String,
        content: Option<String>,
    },
    Rename {
        path: String,
        new_name: String,
    },
    Delete {
        path: String,
    },
    Move {
        path: String,
        new_path: String,
    },
    Copy {
        path: String,
        new_path: String,
    },
    View {
        path: String,
    },
    Update {
        path: String,
        content: String,
    },
}

impl_chain_from!(ServerProjectAction, ServerProjectAction::Io | IoAction::File => FileAction);

impl IsProjectServerAction for FileAction {
    fn with_token(&self) -> bool {
        match self {
            FileAction::Rename { .. }
            | FileAction::Delete { .. }
            | FileAction::Move { .. }
            | FileAction::Copy { .. } => false,
            FileAction::Create { .. } | FileAction::View { .. } | FileAction::Update { .. } => true,
        }
    }

    fn permission(&self) -> Permission {
        match self {
            FileAction::Create { .. }
            | FileAction::Rename { .. }
            | FileAction::Delete { .. }
            | FileAction::Move { .. }
            | FileAction::Copy { .. }
            | FileAction::Update { .. } => Permission::Write,
            FileAction::View { .. } => Permission::Read,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo{
    pub name:String,
    pub content:String,
    pub path:String,
    pub size:u64,
    pub last_modified:String,
}
