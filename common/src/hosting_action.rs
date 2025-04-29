use crate::{ProjectSlugStr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostingAction {
    ServeReloadProject,
    StopServingProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostingActionRequest {
    pub action: HostingAction,
    pub project_slug: ProjectSlugStr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostingActionResponse {
    Ok,
}
