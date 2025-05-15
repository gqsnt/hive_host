use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{impl_chain_from};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectGitAction {
    Pull {
        branch: String,
        commit: String,
        repo_full_name:String,
        token:String,
        
    }
}

impl_chain_from!(ProjectAction, ProjectAction::Git => ProjectGitAction);

impl IsProjectServerAction for ProjectGitAction {
    fn permission(&self) -> Permission {
        match self {
            ProjectGitAction::Pull { .. } => Permission::Write,
        }
    }

    fn require_csrf(&self) -> bool {
        true
    }
}
