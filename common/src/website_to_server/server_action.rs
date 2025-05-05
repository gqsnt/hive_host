pub mod user_action;

use crate::website_to_server::server_action::user_action::ServerUserAction;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize,Serialize)]
pub enum ServerAction {
    UserAction(ServerUserAction),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize,Serialize)]
pub enum ServerActionResponse {
    Ok,
    Error(String),
}
