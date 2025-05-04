
use tracing::{error, info};
use common::hosting::{HostingAction, HostingResponse};
use common::multiplex_protocol::{GenericRequest, GenericResponse};
use common::server::server_to_hosting::{ServerToHostingAction, ServerToHostingResponse};
use crate::{cache_project_path, HostingResult, CACHE};

pub async fn handle_command(request:GenericRequest<ServerToHostingAction>) -> GenericResponse<ServerToHostingResponse> {

    info!("Executing command: {:?}", request.action);
    match execute_command(request.action).await{
        Ok(action_response) => GenericResponse::<ServerToHostingResponse>{
            id: request.id,
            action_response,
        },
        Err(e) =>  {
            error!("Error executing command: {:?}", e);
            GenericResponse::<ServerToHostingResponse>{
                id: request.id,
                action_response: ServerToHostingResponse::Error(e.to_string()),
            }
        }
    }
}



pub async  fn execute_command(action:ServerToHostingAction) -> HostingResult<ServerToHostingResponse> {
    match action{
        ServerToHostingAction::HostingAction(project_slug_str, action) => {
            
            match action {
                HostingAction::ServeReloadProject => {
                    info!("Reloading project {}", project_slug_str);
                    tokio::spawn(cache_project_path(project_slug_str));
                }
                HostingAction::StopServingProject => {
                    CACHE.remove(&project_slug_str);
                }
            }
            Ok(ServerToHostingResponse::HostingActionResponse(HostingResponse::Ok))
        }
        ServerToHostingAction::Ping => {
            Ok(ServerToHostingResponse::Pong)
        }
    }
}
