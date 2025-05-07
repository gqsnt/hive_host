use tarpc::context::Context;
use crate::{cache_project_path, CACHE};
use common::hosting_command::{HostingCommand, HostingResponse};
use tracing::{info};
use common::hosting_command::tarpc::ServerHosting;
use common::ProjectSlugStr;

#[derive(Clone)]
pub struct ServerToHostingServer;

impl ServerHosting for ServerToHostingServer {
    async fn hosting(self, _: Context, project_slug_str: ProjectSlugStr,action: HostingCommand) -> HostingResponse {
        match action {
            HostingCommand::ServeReloadProject => {
                info!("Reloading project {}", project_slug_str);
                tokio::spawn(cache_project_path(project_slug_str));
            }
            HostingCommand::StopServingProject => {
                CACHE.remove(&project_slug_str);
            }
        }
        HostingResponse::Ok
    }
}
