use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, warn};
use crate::multiplex_protocol::{ActionTrait, GenericRequest, GenericResponse, ResponseTrait};

#[derive(Debug, Error)]
pub enum UnixStreamError{
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
}


pub async fn run_unix_socket<F,Fut,Action:ActionTrait,Response:ResponseTrait>(sock_path:String, handle_request:F)
                                                                              ->Result<(), UnixStreamError>
where
    F: Fn(GenericRequest<Action>) -> Fut + Send + Sync + 'static + Clone,
    Fut: Future<Output = GenericResponse<Response>> + Send+ 'static,
{
 
    let listener = UnixListener::bind(sock_path)?;
    info!("Listening for connections on systemd socket...");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let handler_clone = handle_request.clone();
                info!("Accepted connection from {:?}", stream.peer_addr());
                tokio::spawn(process_stream(stream, handler_clone));
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        };
    }
}


pub type UnixStreamResult<T> = Result<T, UnixStreamError>;


async fn process_stream<F,Fut, Action:ActionTrait,Response:ResponseTrait>(
    stream: UnixStream,
    handle_request: F,
) -> UnixStreamResult<()>
where 
    F: Fn(GenericRequest<Action>) -> Fut + Send + Sync+ Clone + 'static,
    Fut: Future<Output = GenericResponse<Response>> + Send+ 'static,
{
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    let mut line = String::new();

    loop {
        line.clear(); // Clear buffer for next read
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - Client closed the connection gracefully
                info!("Client closed the connection.");
                break Ok(()); // Exit the loop cleanly
            }
            Ok(_) => {
                // Successfully read a request line
                let trimmed_line = line.trim();
                if trimmed_line.is_empty() {
                    // Ignore empty lines potentially sent by client? Or treat as error?
                    debug!("Received empty line, continuing read loop.");
                    continue;
                }
                let response = match serde_json::from_str::<GenericRequest<Action>>(trimmed_line){
                    Ok(req) => handle_request(req).await,
                    Err(e) => {
                        debug!("Failed to deserialize JSON: {}", e);
                        let response_json = serde_json::to_string(&GenericResponse::<Response>::get_pong())? + "\n";
                        if write_half
                            .write_all(response_json.as_bytes())
                            .await
                            .is_err()
                        {
                            warn!(
                                "Failed to send deserialization error response to client (connection likely closed)."
                            );
                        }
                        continue; 
                    }
                };
                
                let response_json = serde_json::to_string(&response)? + "\n"; // Add newline delimiter
                match write_half.write_all(response_json.as_bytes()).await {
                    Ok(_) => {
                        // Optional: Flush immediately if needed, though write_all often does enough buffering
                        if let Err(e) = write_half.flush().await {
                            error!(
                                "Failed to flush response to client: {}. Closing connection.",
                                e
                            );
                            break Err(e.into()); // Exit loop with IO error
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to write response to client: {}. Closing connection.",
                            e
                        );
                        // Error writing response likely means client disconnected or pipe broken
                        break Err(e.into()); // Exit loop with IO error
                    }
                }
            }
            Err(e) => {
                // Error reading from the socket
                error!(
                    "Error reading from client connection: {}. Closing connection.",
                    e
                );
                break Err(e.into()); // Exit loop with IO error
            }
        }
    }
}
