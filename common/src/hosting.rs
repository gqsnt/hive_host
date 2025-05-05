use serde::{Deserialize, Serialize};




#[derive(Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum HostingAction {
    ServeReloadProject,
    StopServingProject,
}

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum HostingResponse {
    Ok,
    Error(String),
}
