use crate::ProjectUnixSlugStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostingAction {
    ServeReloadProject,
    StopServingProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostingActionRequest {
    pub action: HostingAction,
    pub project_slug: ProjectUnixSlugStr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostingActionResponse {
    Ok,
}
