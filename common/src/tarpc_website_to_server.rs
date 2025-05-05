use crate::hosting::{HostingAction, HostingResponse};
use crate::website_to_server::server_action::{ServerAction, ServerActionResponse};
use crate::website_to_server::server_project_action::{ServerProjectAction, ServerProjectResponse};

#[tarpc::service]
pub trait WebsiteServer {
    async fn server_action(action: ServerAction) ->ServerActionResponse;
    async fn server_project_action(project_slug: String,action: ServerProjectAction) -> ServerProjectResponse;
    async fn hosting_action(project_slug:String, action: HostingAction) -> HostingResponse;
}
