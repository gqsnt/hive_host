use crate::{cache_project_path, CACHE};
use common::hosting::{HostingAction, HostingResponse};
use tracing::{info};
use common::tarpc_hosting::ServerHosting;

#[derive(Clone)]
pub struct ServerToHostingServer;

impl ServerHosting for ServerToHostingServer {
    async fn execute(
        self,
        _: tarpc::context::Context,
        project_slug: String,
        action: HostingAction,
    ) -> HostingResponse {
        match action {
            HostingAction::ServeReloadProject => {
                info!("Reloading project {}", project_slug);
                tokio::spawn(cache_project_path(project_slug));
            }
            HostingAction::StopServingProject => {
                CACHE.remove(&project_slug);
            }
        }
        HostingResponse::Ok
    }
}
