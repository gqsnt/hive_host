use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, error, info};
use tracing::log::warn;
use common::multiplex_listener::RequestHandler;
use common::multiplex_protocol::{GenericRequest, GenericResponse};
use common::server::server_to_hosting::{ServerToHostingAction, ServerToHostingResponse};
use common::website_to_server::{WebSiteToServerAction, WebSiteToServerResponse};
use crate::{AppState};



#[derive(Clone)]
pub struct ServerRequestHandler {
    pub state: AppState,
}

// Define the connection-specific state
#[derive(Default, Debug)]
pub struct ServerConnectionState {
    is_authenticated: bool,
}


#[async_trait]
impl RequestHandler<WebSiteToServerAction, WebSiteToServerResponse> for ServerRequestHandler {
    type ConnectionState = ServerConnectionState;

    async fn handle_request(
        &self,
        request: GenericRequest<WebSiteToServerAction>,
        conn_state: &mut Self::ConnectionState,
    ) -> GenericResponse<WebSiteToServerResponse> {
        let response_action_result = match request.action {
            WebSiteToServerAction::Auth(token) => {
                if self
                    .get_auth_token()
                    .map_or(false, |t| token == t.expose_secret())
                {
                    info!("Auth successful for Req ID: {}", request.id);
                    conn_state.is_authenticated = true;
                    Ok(WebSiteToServerResponse::Pong) // Indicate success
                } else {
                    warn!("Auth failed for Req ID: {}", request.id);
                    conn_state.is_authenticated = false; // Ensure state is false
                    Ok(WebSiteToServerResponse::Error("Unauthorized".to_string()))
                }
            }
            WebSiteToServerAction::Ping => {
                debug!("Received Ping ID: {}", request.id);
                Ok(WebSiteToServerResponse::Pong)
            }

            // --- Actions requiring authentication ---
            _ if !conn_state.is_authenticated => {
                warn!("Unauthorized action attempt for Req ID: {}", request.id);
                Ok(WebSiteToServerResponse::Error("Unauthorized".to_string()))
            }

            // --- Authenticated Actions ---
            WebSiteToServerAction::ServerAction(sa) => {
                // Map ServerResult<WebSiteToServerResponse> to Result<WebSiteToServerResponse, String>
                match crate::server_action::handle_server_action(self.state.clone(), sa).await {
                    Ok(resp) => Ok(resp),
                    Err(e) => {
                        error!("Error handling ServerAction for ID {}: {}", request.id, e);
                        // Convert ServerError to a response error
                        Ok(WebSiteToServerResponse::Error(e.to_string()))
                    }
                }
            }
            WebSiteToServerAction::ServerProjectAction(project_slug, action) => {
                match crate::project_action::handle_server_project_action(
                    self.state.clone(),
                    project_slug,
                    action,
                    Default::default(), // No StringContent for non-token actions
                )
                    .await {
                    Ok(resp) => Ok(resp),
                    Err(e) => {
                        error!("Error handling ServerProjectAction for ID {}: {}", request.id, e);
                        Ok(WebSiteToServerResponse::Error(e.to_string()))
                    }
                }

            }
            WebSiteToServerAction::HostingAction(project_slug_str, ha) => {
                match self
                    .state
                    .hosting_client
                    .send(ServerToHostingAction::HostingAction(project_slug_str, ha))
                    .await
                {
                    Ok(r) => match r {
                        ServerToHostingResponse::HostingActionResponse(ha_resp) => {
                            Ok(WebSiteToServerResponse::HostingActionResponse(ha_resp))
                        }
                        ServerToHostingResponse::Error(e) => {
                            error!("Hosting service failed for ID {}: {}", request.id, e);
                            Ok(WebSiteToServerResponse::Error(format!(
                                "Hosting service failed: {e}"
                            )))
                        }
                        ServerToHostingResponse::Pong => {
                            warn!("Unexpected Pong from Hosting service for ID {}", request.id);
                            Ok(WebSiteToServerResponse::Error("Unexpected hosting response".to_string()))
                        }
                    },
                    Err(e) => {
                        error!("Client error calling Hosting service for ID {}: {}", request.id, e);
                        Ok(WebSiteToServerResponse::Error(format!("Failed to call hosting service: {e}")))
                    }
                }
            }
        };

        // Construct the final GenericResponse
        let final_response_action = match response_action_result {
            Ok(action) => action,
            Err(e_str) => { // If any handler returned Err(String) directly (shouldn't happen with current mapping)
                error!("Internal handler error for ID {}: {}", request.id, e_str);
                WebSiteToServerResponse::Error(e_str)
            }
        };


        GenericResponse {
            id: request.id,
            action_response: final_response_action,
        }
    }

    fn get_auth_token(&self) -> Option<&SecretString> {
        Some(&self.state.token_auth)
    }
}

