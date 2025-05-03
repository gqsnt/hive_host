use serde::{Deserialize, Serialize};
use crate::hosting::{HostingAction, HostingResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerToHostingAction {
    HostingAction(String, HostingAction),
    Ping,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerToHostingResponse {
    HostingActionResponse(HostingResponse),
    Error(String),
    Pong,
}