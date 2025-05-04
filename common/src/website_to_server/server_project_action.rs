pub mod io_action;
pub mod permission;
pub mod snapshot;

use crate::website_to_server::permission::Permission;
use crate::website_to_server::server_project_action::io_action::dir_action::ServerProjectIoDirActionLsResponse;
use crate::website_to_server::server_project_action::io_action::file_action::FileInfo;
use bitcode::{Decode, Encode};

#[derive(Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum ServerProjectAction {
    Io(io_action::ServerProjectIoAction),
    Permission(permission::ServerProjectPermissionAction),
    Snapshot(snapshot::ServerProjectSnapshotAction),
}

impl IsProjectServerAction for ServerProjectAction {
    fn with_token(&self) -> bool {
        match self {
            ServerProjectAction::Io(action) => action.with_token(),
            ServerProjectAction::Permission(action) => action.with_token(),
            ServerProjectAction::Snapshot(action) => action.with_token(),
        }
    }

    fn permission(&self) -> Permission {
        match self {
            ServerProjectAction::Io(action) => action.permission(),
            ServerProjectAction::Permission(action) => action.permission(),
            ServerProjectAction::Snapshot(action) => action.permission(),
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ServerProjectAction::Io(action) => action.require_csrf(),
            ServerProjectAction::Permission(action) => action.require_csrf(),
            ServerProjectAction::Snapshot(action) => action.require_csrf(),
        }
    }
}

#[derive(Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum ServerProjectResponse {
    Ok,
    Token(String),
    Content(String),
    Ls(ServerProjectIoDirActionLsResponse),
    File(FileInfo),
}

pub trait IsProjectServerAction {
    fn with_token(&self) -> bool;
    fn permission(&self) -> Permission;

    fn require_csrf(&self) -> bool;
}
