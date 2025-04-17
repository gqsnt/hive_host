pub mod permission;
pub mod io_action;


use serde::{Deserialize, Serialize};
use crate::permission::Permission;
use crate::{ProjectSlug, UserSlug};
use crate::server_project_action::io_action::dir_action::DirActionTreeResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProjectActionRequest {
    pub token:Option<String>,
    pub project_slug:ProjectSlug,
    pub action:ServerProjectAction,
}



#[derive(Serialize, Deserialize, Debug,Clone)]
pub enum ServerProjectAction{
    Create{
        owner_slug:UserSlug,
    },
    Io(io_action::IoAction),
    Permission(permission::PermissionAction),
}

impl IsProjectServerAction for ServerProjectAction{
    fn with_token(&self) -> bool {
        match self{
            ServerProjectAction::Create { .. } => false,
            ServerProjectAction::Io(action) => action.with_token(),
            ServerProjectAction::Permission(action) => action.with_token()
        }
    }

    fn permission(&self) -> Permission {
        match self{
            ServerProjectAction::Create { .. } => Permission::Owner,
            ServerProjectAction::Io(action) => action.permission(),
            ServerProjectAction::Permission(action) =>action.permission()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerProjectActionResponse{
    Ok,
    Token(String),
    Content(String),
    Tree(DirActionTreeResponse),
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct ProjectActionCreate{
    pub project_slug:ProjectSlug,
    pub owner_slug:UserSlug,
}

pub trait IsProjectServerAction {
    fn with_token(&self) -> bool ;
    fn permission(&self) -> Permission ;

}
