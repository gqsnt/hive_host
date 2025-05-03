
pub mod server_action;

pub mod server_project_action;

pub mod permission;


use serde::{Deserialize, Serialize};
use crate::hosting::{HostingAction, HostingResponse};
use crate::ProjectSlugStr;
use crate::website_to_server::server_action::{ServerAction, ServerActionResponse};
use crate::website_to_server::server_project_action::{ServerProjectAction, ServerProjectResponse};




#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum WebSiteToServerAction {
    ServerAction(ServerAction),
    ServerProjectAction(ProjectSlugStr, ServerProjectAction),
    HostingAction(ProjectSlugStr, HostingAction),
    Auth(String),
    Ping,
}


impl WebSiteToServerAction {
    pub fn from_auth(token:String) -> Self{
        WebSiteToServerAction::Auth(token)
    }
    pub fn from_server_project_action(project_slug:ProjectSlugStr, action:ServerProjectAction) -> Self{
        WebSiteToServerAction::ServerProjectAction(project_slug, action)
    }

    pub fn from_hosting_action(project_slug:ProjectSlugStr, action: HostingAction) -> Self{
        WebSiteToServerAction::HostingAction(project_slug, action)
    }
}


impl From<ServerAction> for WebSiteToServerAction {
    fn from(value :ServerAction) -> Self {
        WebSiteToServerAction::ServerAction(value)
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum WebSiteToServerResponse {
    HostingActionResponse(HostingResponse),
    ServerActionResponse(ServerActionResponse),
    ServerProjectActionResponse(ServerProjectResponse),
    Pong,
    Error(String)
}


impl From<HostingResponse> for WebSiteToServerResponse {
    fn from(value : HostingResponse) -> Self {
        WebSiteToServerResponse::HostingActionResponse(value)
    }
}

impl From<ServerActionResponse> for WebSiteToServerResponse {
    fn from(value :ServerActionResponse) -> Self {
        WebSiteToServerResponse::ServerActionResponse(value)
    }
}

impl From<ServerProjectResponse> for WebSiteToServerResponse {
    fn from(value : ServerProjectResponse) -> Self {
        WebSiteToServerResponse::ServerProjectActionResponse(value)
    }
}
