pub mod io_action;
pub mod permission;
pub mod snapshot;

use crate::permission::Permission;
use crate::server_project_action::io_action::dir_action::DirActionLsResponse;
use crate::server_project_action::io_action::file_action::FileInfo;
use crate::Slug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProjectActionRequest {
    pub token: Option<String>,
    pub project_slug: Slug,
    pub action: ServerProjectAction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerProjectAction {
    Io(io_action::IoAction),
    Permission(permission::PermissionAction),
    Snapshot(snapshot::SnapshotAction),
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerProjectActionResponse {
    Ok,
    Token(String),
    Content(String),
    Ls(DirActionLsResponse),
    File(FileInfo),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectActionCreate {
    pub project_slug: Slug,
    pub owner_slug: Slug,
}

pub trait IsProjectServerAction {
    fn with_token(&self) -> bool;
    fn permission(&self) -> Permission;

    fn require_csrf(&self) -> bool;
}
