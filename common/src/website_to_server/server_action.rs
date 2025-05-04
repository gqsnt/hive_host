pub mod user_action;

use crate::website_to_server::server_action::user_action::ServerUserAction;
use bitcode::{Decode, Encode};

#[derive(Decode,Encode, Debug, Clone, PartialEq, Eq)]
pub enum ServerAction {
    UserAction(ServerUserAction),
}

#[derive(Decode,Encode, Debug, Clone, PartialEq, Eq)]
pub enum ServerActionResponse {
    Ok,
}
