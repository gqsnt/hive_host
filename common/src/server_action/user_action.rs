use crate::{
    GitBranchNameStr, GitRepoFullNameStr, GitTokenStr, ProjectSlugStr, SanitizeError, UserSlugStr,
    Validate,
};

use crate::helper_command::HelperResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ServerUserAction {
    Create {
        user_slug: UserSlugStr,
    },
    AddProject {
        user_slug: UserSlugStr,
        project_slug: ProjectSlugStr,
        github_info: Option<(Option<GitTokenStr>, GitRepoFullNameStr, GitBranchNameStr)>,
    },
    RemoveProject {
        user_slugs: Vec<UserSlugStr>,
        project_slug: ProjectSlugStr,
    },
    Delete {
        user_slug: UserSlugStr,
    },
}

impl Validate for ServerUserAction {
    fn validate(&self) -> Result<(), SanitizeError> {
        match self {
            ServerUserAction::Create { user_slug } => {
                user_slug.validate()?;
            }
            ServerUserAction::AddProject {
                user_slug,
                project_slug,
                github_info,
            } => {
                user_slug.validate()?;
                project_slug.validate()?;
                if let Some((token, repo, branch)) = github_info {
                    repo.validate()?;
                    branch.validate()?;
                    if let Some(token) = token {
                        token.validate()?;
                    }
                }
            }
            ServerUserAction::RemoveProject {
                user_slugs,
                project_slug,
            } => {
                project_slug.validate()?;
                for user_slug in user_slugs {
                    user_slug.validate()?;
                }
            }
            ServerUserAction::Delete { user_slug } => {
                user_slug.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ServerUserResponse {
    Ok,
    Helper(HelperResponse),
    Error(String),
}
