use bitcode::{Decode, Encode};
use crate::hosting::{HostingAction, HostingResponse};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum ServerToHostingAction {
    HostingAction(String, HostingAction),
    Ping,
}


#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum ServerToHostingResponse {
    HostingActionResponse(HostingResponse),
    Error(String),
    Pong,
}