use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error, info, warn};
use common::server_helper::{ServerHelperRequest, ServerHelperResponse, ServerHelperResponseStatus};
use crate::command::execute_command;
use crate::ServerHelperResult;


pub async fn handle_connection(stream: UnixStream) {
    info!("Handling new client connection");
    match process_stream(stream).await {
        Ok(_) => info!("Client connection handled successfully"),
        Err(e) => error!("Error processing client connection: {:?}", e), 
    }
    info!("Client connection handler task finished."); 
}

async fn process_stream(stream: UnixStream) -> ServerHelperResult<()> {
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

                // Deserialize request
                let request: ServerHelperRequest = match serde_json::from_str(trimmed_line) {
                    Ok(req) => req,
                    Err(e) => {
                        error!("Failed to deserialize request: {}. Raw: '{}'", e, trimmed_line);
                        let response = ServerHelperResponse { status: ServerHelperResponseStatus::Error(format!("Bad request: {}", e)) };
                        let response_json = serde_json::to_string(&response)? + "\n";
                        // Try to send error back before potentially closing
                        if write_half.write_all(response_json.as_bytes()).await.is_err() {
                            warn!("Failed to send deserialization error response to client (connection likely closed).");
                        }
                        // Maybe close connection on bad request? Or just continue loop?
                        // Let's continue for now, maybe client sends another request.
                        continue; // Continue loop after sending error
                    }
                };
                

                // Execute command
                let response_status = match execute_command(request.command).await {
                    Ok(_) => ServerHelperResponseStatus::Success,
                    Err(e) => {
                        error!("Command execution failed: {:?}", e);
                        ServerHelperResponseStatus::Error(format!("Command execution failed: {}", e.to_string()))
                    }
                };

                // Serialize and send response
                let response = ServerHelperResponse { status: response_status };
                let response_json = serde_json::to_string(&response)? + "\n"; // Add newline delimiter

                match write_half.write_all(response_json.as_bytes()).await {
                    Ok(_) => {
                        // Optional: Flush immediately if needed, though write_all often does enough buffering
                        if let Err(e) = write_half.flush().await { 
                            error!("Failed to flush response to client: {}. Closing connection.", e);
                            break Err(e.into()); // Exit loop with IO error
                        }
                        info!("Response sent successfully: {:?}", response);
                    }
                    Err(e) => {
                        error!("Failed to write response to client: {}. Closing connection.", e);
                        // Error writing response likely means client disconnected or pipe broken
                        break Err(e.into()); // Exit loop with IO error
                    }
                }
            }
            Err(e) => {
                // Error reading from the socket
                error!("Error reading from client connection: {}. Closing connection.", e);
                break Err(e.into()); // Exit loop with IO error
            }
        }
    } 
}