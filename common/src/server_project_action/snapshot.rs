use crate::impl_chain_from;
use crate::permission::Permission;
use crate::server_project_action::{IsProjectServerAction, ServerProjectAction};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SnapshotAction {
    Create { snapshot_name: String },
    Delete { snapshot_name: String },
    MountSnapshotProd { snapshot_name: String },
    UnmountProd,
}

impl_chain_from!(ServerProjectAction, ServerProjectAction::Snapshot => SnapshotAction);

impl IsProjectServerAction for SnapshotAction {
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
