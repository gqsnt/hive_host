use crate::impl_chain_from;
use crate::website_to_server::permission::Permission;
use crate::website_to_server::server_project_action::{IsProjectServerAction, ServerProjectAction};
use serde::{Deserialize, Serialize};

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum ServerProjectSnapshotAction {
    Create { snapshot_name: String },
    Delete { snapshot_name: String },
    MountSnapshotProd { snapshot_name: String },
    UnmountProd,
}

impl_chain_from!(ServerProjectAction, ServerProjectAction::Snapshot => ServerProjectSnapshotAction);

impl IsProjectServerAction for ServerProjectSnapshotAction {
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
