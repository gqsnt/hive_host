use std::net::SocketAddr;
use secrecy::ExposeSecret;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use common::multiplex_protocol::{GenericRequest, GenericResponse};
use common::server::server_to_hosting::{ServerToHostingAction, ServerToHostingResponse};
use common::website_to_server::{WebSiteToServerAction, WebSiteToServerResponse};
use crate::{AppState, ServerError};

pub async fn run_tcp_server(state: AppState, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("TCP server listening on {}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::info!("TCP connection accepted from: {}", peer_addr);
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_tcp_connection(stream, state_clone).await {
                tracing::error!("Error handling TCP connection from {}: {:?}", peer_addr, e);
            }
        });
    }
}


async fn handle_tcp_connection(mut stream: tokio::net::TcpStream, state: AppState) -> Result<(), ServerError> {
    let mut is_authenticated = false; // State per connection

    loop {
        // 1. Read Length
        let len = match stream.read_u32().await {
            Ok(0) => { tracing::info!("TCP Connection closed (read 0 length)"); return Ok(()); }
            Ok(len) => len,
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                tracing::info!("TCP Connection closed (EOF reading length)"); return Ok(());
            }
            Err(e) => { tracing::error!("TCP Failed to read length: {}", e); return Err(e.into()); }
        };
        if len == 0 || len > 10 * 1024 * 1024 { // Add a sanity check for max message size (e.g., 10MB)
            tracing::error!("TCP Received invalid length: {}", len);
            return Err(ServerError::InvalidMessageLength); // Use your error type
        }


        // 2. Read Payload
        let mut buffer = vec![0u8; len as usize];
        if let Err(e) = stream.read_exact(&mut buffer).await {
            tracing::error!("TCP Failed to read payload: {}", e);
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                tracing::info!("TCP Connection closed (EOF reading payload)");
            }
            return Err(e.into());
        }

        // 3. Deserialize Request
        let request: GenericRequest<WebSiteToServerAction> = match serde_json::from_slice(&buffer) {
            Ok(req) => req,
            Err(e) => {
                tracing::error!("TCP Failed to deserialize request: {}", e);
                // Cannot respond meaningfully without request ID, close connection
                return Err(ServerError::DeserializationError(e));
            }
        };
        tracing::debug!("TCP Received Req ID: {}", request.id);


        // --- Process Actions ---
        let response_action = match request.action {
            WebSiteToServerAction::Auth ( token) => { 
                if token == state.token_auth.expose_secret() {
                    tracing::info!("TCP Auth successful for Req ID: {}", request.id);
                    is_authenticated = true;
                    // Need a success response variant if protocol requires one, otherwise Pong might suffice?
                    // Let's assume Pong indicates success for simplicity here.
                    Ok(WebSiteToServerResponse::Pong)
                } else {
                    tracing::warn!("TCP Auth failed for Req ID: {}", request.id);
                    is_authenticated = false;
                    Ok(WebSiteToServerResponse::Error("Unauthorized".to_string()))
                }
            }
            WebSiteToServerAction::Ping => {
                tracing::debug!("TCP Received Ping ID: {}", request.id);
                Ok(WebSiteToServerResponse::Pong)
            }
            // --- Actions requiring authentication ---
            _ if !is_authenticated => {
                tracing::warn!("TCP Unauthorized action attempt for Req ID: {}", request.id);
                Ok(WebSiteToServerResponse::Error("Unauthorized".to_string()))
            }
            // --- Authenticated Actions ---
            WebSiteToServerAction::ServerAction(sa) => {
                crate::server_action::handle_server_action(state.clone(), sa).await
            }
            WebSiteToServerAction::ServerProjectAction(project_slug, action) => {
                crate::project_action::handle_server_project_action(
                    state.clone(),
                    project_slug, // Assuming slug is string here
                    action,
                    Default::default(), // No StringContent for non-token actions
                ).await
            }
            WebSiteToServerAction::HostingAction(project_slug_str, ha) => {
                let r=state.hosting_client.send(
                    ServerToHostingAction::HostingAction(project_slug_str, ha)
                ).await?;
                match r{
                    ServerToHostingResponse::HostingActionResponse(ha) => {
                        Ok(WebSiteToServerResponse::HostingActionResponse(ha))
                    }
                    ServerToHostingResponse::Error(e) => {
                        tracing::error!("TCP Failed to process HostingAction for ID {}: {}", request.id, e);
                        Ok(WebSiteToServerResponse::Error(format!("Failed to process HostingAction: {}", e)))
                    }
                    _ => {
                        tracing::error!("TCP Unexpected HostingAction response for ID {}: {:?}", request.id, r);
                        Ok(WebSiteToServerResponse::Error("Unexpected HostingAction response".to_string()))
                    }
                }
            }
        };
        
        let response_action = match response_action {
            Ok(action) => action,
            Err(e) => {
                tracing::error!("TCP Failed to process action for ID {}: {}", request.id, e);
                WebSiteToServerResponse::Error(format!("Failed to process action: {}", e))
            }
        };

        // --- Send Response ---
        let response = GenericResponse::<WebSiteToServerResponse> {
            id: request.id,
            action_response: response_action,
        };

        let response_bytes = match serde_json::to_vec(&response) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("TCP Failed to serialize response for ID {}: {}", request.id, e);
                // Don't close connection, just log and try next request maybe?
                continue;
            }
        };

        if let Err(e) = stream.write_u32(response_bytes.len() as u32).await {
            tracing::error!("TCP Failed to write response length for ID {}: {}", request.id, e);
            return Err(e.into());
        }

        if let Err(e) = stream.write_all(&response_bytes).await {
            tracing::error!("TCP Failed to write response payload for ID {}: {}", request.id, e);
            return Err(e.into());
        }
        tracing::debug!("TCP Sent Resp ID: {}", response.id);
    }
}

