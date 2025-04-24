use serde::{Deserialize, Serialize};
use crate::ProjectUnixSlugStr;

#[derive(Debug, Clone, PartialEq, Eq,Serialize, Deserialize)]
pub enum HostingAction{
    ServeReloadProject,
    StopServingProject,
}


#[derive(Debug, Clone, PartialEq, Eq,Serialize, Deserialize)]
pub struct HostingActionRequest{
    pub action: HostingAction,
    pub project_slug: ProjectUnixSlugStr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostingActionResponse {
    Ok,
}
