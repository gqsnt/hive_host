use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{Slug, impl_chain_from};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectGitAction {
    Pull {
        branch: String,
        clean_untracked: bool,
    },
    PullWithMountToProd {
        branch: String,
        clean_untracked: bool,
        snapshot_name: String,
        should_umount_first: bool,
    },
}

impl_chain_from!(ProjectAction, ProjectAction::Git => ProjectGitAction);

impl IsProjectServerAction for ProjectGitAction {
    fn permission(&self) -> Permission {
        match self {
            ProjectGitAction::Pull { .. } => Permission::Write,
            ProjectGitAction::PullWithMountToProd { .. } => Permission::Owner,
        }
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
