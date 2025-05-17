use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{SanitizeError, SnapShotNameStr, Validate, impl_chain_from};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectSnapshotAction {
    Create {
        snapshot_name: SnapShotNameStr,
    },
    Delete {
        snapshot_name: SnapShotNameStr,
    },
    Restore {
        snapshot_name: SnapShotNameStr,
    },
    MountSnapshotProd {
        snapshot_name: SnapShotNameStr,
        should_umount_first: bool,
    },
    UnmountProd,
}

impl Validate for ProjectSnapshotAction {
    fn validate(&self) -> Result<(), SanitizeError> {
        match self {
            ProjectSnapshotAction::Create { snapshot_name } => {
                snapshot_name.validate()?;
            }
            ProjectSnapshotAction::Delete { snapshot_name } => {
                snapshot_name.validate()?;
            }
            ProjectSnapshotAction::Restore { snapshot_name } => {
                snapshot_name.validate()?;
            }
            ProjectSnapshotAction::MountSnapshotProd {
                snapshot_name,
                should_umount_first: _,
            } => {
                snapshot_name.validate()?;
            }
            ProjectSnapshotAction::UnmountProd => {}
        }
        Ok(())
    }
}

impl_chain_from!(ProjectAction, ProjectAction::Snapshot => ProjectSnapshotAction);

impl IsProjectServerAction for ProjectSnapshotAction {
    fn permission(&self) -> Permission {
        Permission::Owner
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
