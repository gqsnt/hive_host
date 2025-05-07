pub mod io_action;
pub mod permission;
pub mod snapshot;

use crate::server_action::permission::Permission;
use crate::server_action::project_action::io_action::dir_action::ServerProjectIoDirActionLsResponse;
use crate::server_action::project_action::io_action::file_action::FileInfo;

use serde::{Deserialize, Serialize};
use crate::helper_command::HelperResponse;
use crate::hosting_command::HostingResponse;

#[derive(Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectAction {
    Io(io_action::ProjectIoAction),
    Permission(permission::ProjectPermissionAction),
    Snapshot(snapshot::ProjectSnapshotAction),
}

impl IsProjectServerAction for ProjectAction {
    fn with_token(&self) -> bool {
        match self {
            ProjectAction::Io(action) => action.with_token(),
            ProjectAction::Permission(action) => action.with_token(),
            ProjectAction::Snapshot(action) => action.with_token(),
        }
    }

    fn permission(&self) -> Permission {
        match self {
            ProjectAction::Io(action) => action.permission(),
            ProjectAction::Permission(action) => action.permission(),
            ProjectAction::Snapshot(action) => action.permission(),
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            ProjectAction::Io(action) => action.require_csrf(),
            ProjectAction::Permission(action) => action.require_csrf(),
            ProjectAction::Snapshot(action) => action.require_csrf(),
        }
    }
}

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectResponse {
    Ok,
    Error(String),
    HelperResponses(HelperResponse),
    HostingResponse(HostingResponse),
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
