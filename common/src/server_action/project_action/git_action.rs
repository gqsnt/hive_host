use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction, ProjectAction};
use crate::{
    GitBranchNameStr, GitCommitStr, GitRepoFullNameStr, GitTokenStr, Validate, impl_chain_from,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProjectGitAction {
    Pull {
        branch: GitBranchNameStr,
        commit: GitCommitStr,
        repo_full_name: GitRepoFullNameStr,
        token: GitTokenStr,
    },
}

impl Validate for ProjectGitAction {
    fn validate(&self) -> Result<(), crate::SanitizeError> {
        match self {
            ProjectGitAction::Pull {
                branch,
                commit,
                repo_full_name,
                token,
            } => {
                branch.validate()?;
                commit.validate()?;
                repo_full_name.validate()?;
                token.validate()?;
            }
        }
        Ok(())
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
