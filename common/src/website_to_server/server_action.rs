pub mod user_action;

use crate::website_to_server::server_action::user_action::ServerUserAction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerAction {
    UserAction(ServerUserAction),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerActionResponse {
    Ok,
}
