use async_trait::async_trait;
use secrecy::SecretString;

use thiserror::Error;
use tracing::{debug, error, info, warn};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::multiplex_protocol::{ActionTrait, GenericRequest, GenericResponse, ResponseTrait};
#[cfg(feature = "multiplex-listener-tcp")]
use tokio::net::TcpListener;

#[cfg(feature = "multiplex-listener-unix")]
use tokio::net::UnixListener;

#[derive(Debug, Error)]
pub enum MultiplexListenerError{
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization/Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Failed to bind listener: {0}")]
    BindError(String),
    #[error("Listener configuration error: {0}")]
    ConfigError(String),
    #[error("Feature mismatch: Required listener feature not enabled.")]
    FeatureError,
    #[error("Internal processing error: {0}")]
    ProcessingError(String),
}


#[async_trait]
pub trait RequestHandler<Action, Response>: Send + Sync + Clone + 'static
where
    Action: ActionTrait,
    Response: ResponseTrait,
{
    /// Represents the state specific to a single connection (e.g., authentication status).
    /// Must implement Default, Send, and Sync.
    type ConnectionState: Default + Send + Sync;

    /// Handles an incoming request.
    ///
    /// # Arguments
    /// * `request`: The deserialized generic request.
    /// * `conn_state`: Mutable reference to the connection-specific state. Modify this
    ///   to manage state like authentication across requests on the same connection.
    ///
    /// # Returns
    /// A `GenericResponse` containing the result of processing the action.
    async fn handle_request(
        &self,
        request: GenericRequest<Action>,
        conn_state: &mut Self::ConnectionState,
    ) -> GenericResponse<Response>;

    /// Optional: Method to get the authentication token (relevant for TCP).
    /// The listener uses this to validate incoming Auth actions.
    fn get_auth_token(&self) -> Option<&SecretString> {
        None
    }
}

async fn process_connection<R, W, H, Action, Response>(
    mut reader: R,
    mut writer: W,
    handler: H,
    peer_desc: String, // Description for logging (e.g., peer address)
) -> Result<(), MultiplexListenerError>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
    H: RequestHandler<Action, Response>,
    Action: ActionTrait,
    Response: ResponseTrait,
{
    info!("Processing connection from {}", peer_desc);
    let mut conn_state = H::ConnectionState::default();

    loop {
        // 1. Read Length Prefix
        let len = match reader.read_u32().await {
            Ok(0) => {
                info!("Connection closed (read 0 length) from {}", peer_desc);
                return Ok(());
            }
            Ok(len) => len,
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                info!("Connection closed cleanly (EOF reading length) from {}", peer_desc);
                return Ok(());
            }
            Err(e) => {
                error!("Failed to read length prefix from {}: {}", peer_desc, e);
                return Err(e.into());
            }
        };

        // Sanity check message size
        if len > 10 * 1024 * 1024 { // e.g., 10MB limit
            error!("Received excessive length ({}) from {}", len, peer_desc);
            // Consider sending an error response before closing? Difficult without request ID.
            return Err(MultiplexListenerError::ProcessingError(format!(
                "Received excessive length: {}",
                len
            )));
        }
        if len == 0 {
            info!("Received zero length message body from {}. Assuming EOF/closed.", peer_desc);
            return Ok(()); // Treat length 0 as connection closed after length read
        }

        // 2. Read Payload
        let mut buffer = vec![0u8; len as usize];
        if let Err(e) = reader.read_exact(&mut buffer).await {
            error!("Failed to read payload from {}: {}", peer_desc, e);
            return Err(e.into());
        }

        // 3. Deserialize Request
        let request: GenericRequest<Action> = match serde_json::from_slice(&buffer) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to deserialize request from {}: {}", peer_desc, e);
                // Attempt to send an error response back if possible
                let error_response = GenericResponse::<Response>::make_error(
                    0, // No specific request ID known
                    format!("Deserialization failed: {}", e),
                );
                match serde_json::to_vec(&error_response) {
                    Ok(error_bytes) => {
                        if writer.write_u32(error_bytes.len() as u32).await.is_ok() {
                            if writer.write_all(&error_bytes).await.is_err() {
                                warn!("Failed to send deserialization error response (write_all failed) to {}", peer_desc);
                            }
                        } else {
                            warn!("Failed to send deserialization error response (write_u32 failed) to {}", peer_desc);
                        }
                    }
                    Err(serr) => {
                        error!("Failed to serialize the error response itself: {}", serr);
                    }
                }
                // Continue processing other requests on this connection? Or close it?
                // Let's close the connection on deserialization error for safety.
                return Err(e.into());
            }
        };

        debug!("Received Req ID {} from {}", request.id, peer_desc);

        // 4. Handle Request using the provided handler
        let response = handler.handle_request(request, &mut conn_state).await;

        // 5. Serialize Response
        let response_bytes = match serde_json::to_vec(&response) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to serialize response ID {} for {}: {}", response.id, peer_desc, e);
                // Don't close connection, just log and try next request?
                // Or send a generic error response if possible? Difficult.
                continue; // Let's attempt to continue
            }
        };

        // 6. Write Length Prefix for Response
        if let Err(e) = writer.write_u32(response_bytes.len() as u32).await {
            error!("Failed to write response length for ID {} to {}: {}", response.id, peer_desc, e);
            return Err(e.into());
        }

        // 7. Write Response Payload
        debug!("Sending Resp ID {} ({} bytes) to {}", response.id, response_bytes.len(), peer_desc);
        if let Err(e) = writer.write_all(&response_bytes).await {
            error!("Failed to write response payload for ID {} to {}: {}", response.id, peer_desc, e);
            return Err(e.into());
        }

        // Optional: Flush if needed, though write_all often suffices
        // if let Err(e) = writer.flush().await {
        //     error!("Failed to flush write buffer for {}: {}", peer_desc, e);
        //     return Err(e.into());
        // }
    }
}

#[cfg(feature = "multiplex-listener-tcp")]
pub async fn run_server_tcp<H, Action, Response>(
    addr: String,
    handler: H,
) -> Result<(), MultiplexListenerError>
where
    H: RequestHandler<Action, Response>,
    Action: ActionTrait,
    Response: ResponseTrait,
{

    // #[cfg(all(feature = "multiplex-listener-unix", feature = "multiplex-listener-tcp"))]
    // compile_error!("Features 'multiplex-listener-unix' and 'multiplex-listener-tcp' are mutually exclusive.");

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| MultiplexListenerError::BindError(e.to_string()))?;
    info!("TCP server listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let handler_clone = handler.clone();
                let peer_desc = peer_addr.to_string();
                info!("TCP connection accepted from: {}", peer_desc);
                tokio::spawn(async move {
                    let (reader, writer) = tokio::io::split(stream);
                    if let Err(e) = process_connection(reader, writer, handler_clone, peer_desc.clone()).await {
                        error!("Error processing TCP connection from {}: {:?}", peer_desc, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept TCP connection: {}", e);
                // Avoid busy-looping on accept errors
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}


#[cfg(feature = "multiplex-listener-unix")]
pub async fn run_server_unix<H, Action, Response>(
    path: String,
    handler: H,
) -> Result<(), MultiplexListenerError>
where
    H: RequestHandler<Action, Response>,
    Action: ActionTrait,
    Response: ResponseTrait,
{

    // #[cfg(all(feature = "multiplex-listener-unix", feature = "multiplex-listener-tcp"))]
    // compile_error!("Features 'multiplex-listener-unix' and 'multiplex-listener-tcp' are mutually exclusive.");

    // Attempt to remove existing socket file before binding
    if let Err(e) = tokio::fs::remove_file(&path).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            warn!("Failed to remove existing socket file at {}: {}", path, e);
            // Decide if this should be a hard error or just a warning
            return Err(MultiplexListenerError::Io(e)); // Let's make it an error
        }
    }

    let listener = UnixListener::bind(&path)
        .map_err(|e| MultiplexListenerError::BindError(e.to_string()))?;
    info!("Unix socket server listening on {}", path);

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => { // Unix addr is often less useful/standardized
                let handler_clone = handler.clone();
                // Use path for peer description for Unix sockets
                let peer_desc = format!("unix:{}", path); // Use the bound path
                info!("Unix socket connection accepted");
                tokio::spawn(async move {
                    let (reader, writer) = tokio::io::split(stream);
                    if let Err(e) = process_connection(reader, writer, handler_clone, peer_desc.clone()).await {
                        error!("Error processing Unix connection for {}: {:?}", peer_desc, e);
                    } else {
                        info!("Unix connection finished for {}", peer_desc);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept Unix connection: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}