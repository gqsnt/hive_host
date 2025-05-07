use crate::impl_chain_from;
use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use serde::{Deserialize, Serialize};

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ProjectSnapshotAction {
    Create { snapshot_name: String },
    Delete { snapshot_name: String },
    MountSnapshotProd { snapshot_name: String, should_umount_first:bool },
    UnmountProd,
}

impl_chain_from!(ProjectAction, ProjectAction::Snapshot => ProjectSnapshotAction);

impl IsProjectServerAction for ProjectSnapshotAction {
    fn with_token(&self) -> bool {
        false
    }

    fn permission(&self) -> Permission {
        Permission::Owner
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
