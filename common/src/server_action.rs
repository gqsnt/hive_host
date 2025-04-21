pub mod user_action;

use crate::server_action::user_action::UserAction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerAction {
    UserAction(UserAction),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerActionResponse {
    Ok,
}
