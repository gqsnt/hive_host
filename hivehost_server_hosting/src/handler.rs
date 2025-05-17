use secrecy::ExposeSecret;
use tarpc::context::Context;
use crate::{cache_project_path, AppState, CACHE};
use common::hosting_command::{HostingCommand, HostingResponse};
use tracing::{info};
use common::hosting_command::tarpc::ServerHosting;
use common::{AuthResponse, AuthToken, ProjectSlugStr, Validate};

#[derive(Clone)]
pub struct ServerToHostingServer(pub AppState);

impl ServerHosting for ServerToHostingServer {
    async fn hosting(self, _: Context, project_slug_str: ProjectSlugStr,action: HostingCommand) -> HostingResponse {
        match project_slug_str.validate() {
            Ok(_) => {}
            Err(e) => {
                return HostingResponse::Error(format!("Invalid action: {e}"));
            }
        };
        match action {
            HostingCommand::ServeReloadProject => {
                info!("Reloading project {:?}", project_slug_str);
                tokio::spawn(cache_project_path(project_slug_str));
            }
            HostingCommand::StopServingProject => {
                CACHE.remove(&project_slug_str);
            }
        }
        HostingResponse::Ok
    }

    async fn auth(self, _: Context, token: AuthToken) -> AuthResponse {
        let mut connected= self.0.connected.write().await;
        if self.0.server_auth.expose_secret().eq(&token.0){
            info!("Token auth success");
            *connected = true;
            AuthResponse::Ok
        }else{
            *connected = false;
            info!("Token auth failed");
            AuthResponse::Error
        }

    }
}
