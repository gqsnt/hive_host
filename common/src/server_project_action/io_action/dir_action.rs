use crate::permission::Permission;
use serde::{Deserialize, Serialize};
use crate::impl_chain_from;
use crate::server_project_action::io_action::IoAction;
use crate::server_project_action::{IsProjectServerAction, ServerProjectAction};



#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DirAction{
    Create{path:String},
    Rename{path:String, new_name:String},
    Delete{path:String},
    Ls{path:String},
    Download,
}


impl_chain_from!(ServerProjectAction , ServerProjectAction::Io | IoAction::Dir  => DirAction);


impl IsProjectServerAction for DirAction{
    fn with_token(&self) -> bool {
        match self{
            DirAction::Create { .. }
            |DirAction::Rename { .. }
            |DirAction::Delete { .. }
            |DirAction::Ls{..} => false,
            DirAction::Download => true
        }
    }

    fn permission(&self) -> Permission {
        match  self{
            DirAction::Create { .. }
            |DirAction::Rename { .. }
            |DirAction::Delete { .. } => Permission::Write,
            DirAction::Download
            |DirAction::Ls{..} =>  Permission::Read
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirActionLsResponse {
    pub inner: Vec<LsElement>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LsElement {
    pub name: String,
    pub is_dir: bool,
}


