
pub mod server_action;

pub mod server_project_action;

pub mod permission;



use crate::server::server_to_helper::ServerToHelperResponse;
use crate::website_to_server::server_project_action::{ServerProjectResponse};



impl From<ServerToHelperResponse> for ServerProjectResponse{
    fn from(value: ServerToHelperResponse) -> Self {
        match value {
            ServerToHelperResponse::Ok => ServerProjectResponse::Ok,
            ServerToHelperResponse::Error(e) => ServerProjectResponse::Error(e.to_string()),
        }
    }
}