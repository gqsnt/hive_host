use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tracing::{debug, error, instrument};
use tracing::log::info;
use common::server_helper::{ServerHelperCommand, ServerHelperRequest, ServerHelperResponse, ServerHelperResponseStatus};



#[derive(Error, Debug)]
pub enum HelperClientError {
    #[error("Failed to connect to helper service: {0}")]
    ConnectionFailed(#[source] std::io::Error),

    #[error("Failed to send command to helper service: {0}")]
    SendError(#[from] mpsc::error::SendError<(ServerHelperCommand, ResponseTx)>),

    #[error("Helper service disconnected or panicked")]
    ServiceDisconnected,

    #[error("Helper service returned an error: {0}")]
    ServiceError(String),

    #[error("Failed to serialize command: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Failed to deserialize response: {0}")]
    DeserializationError(#[source] serde_json::Error),

    #[error("I/O error during communication: {0}")]
    IoError(#[source] std::io::Error),

    #[error("Internal client error: {0}")]
    InternalError(String),
}

pub type HelperClientResult<T> = Result<T, HelperClientError>;
pub(crate) type ResponseTx = oneshot::Sender<HelperClientResult<()>>;
// Type alias for the command channel sender
pub(crate) type CommandTx = mpsc::Sender<(ServerHelperCommand, ResponseTx)>;
// Type alias for the command channel receiver
pub(crate) type CommandRx = mpsc::Receiver<(ServerHelperCommand, ResponseTx)>;


const RECONNECT_DELAY_MS: u64 = 500;
const MAX_RECONNECT_DELAY_MS: u64 = 8000; // Approx 8 seconds

#[derive(Clone, Debug)]
pub struct HelperClient {
    sender: CommandTx,
}

impl HelperClient {
    /// Creates a new client handle. Should only be called once during setup.
    fn new(sender: CommandTx) -> Self {
        HelperClient { sender }
    }

    /// Sends a command to the helper service and waits for the result.
    #[instrument(skip(self), fields(command = ?command))]
    pub async fn execute(&self, command: ServerHelperCommand) -> HelperClientResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        self.sender.send((command, response_tx)).await?;

        response_rx.await.unwrap_or_else(|_| {
            // This happens if the actor task panics or disconnects unexpectedly
            error!("HelperClient actor task disconnected.");
            Err(HelperClientError::ServiceDisconnected)
        })
    }
}


pub struct HelperClientActor {
    socket_path: String,
    receiver: CommandRx,
    // Store writer/reader separately to allow reconnection
    writer: Option<BufWriter<OwnedWriteHalf>>,
    reader: Option<BufReader<OwnedReadHalf>>, // Reader needs separate ownership for read_line
}

impl HelperClientActor {
    fn new(socket_path: String, receiver: CommandRx) -> Self {
        HelperClientActor {
            socket_path,
            receiver,
            writer: None,
            reader: None,
        }
    }

    /// Runs the actor's main loop.
    pub async fn run(mut self) {
        info!("HelperClientActor started. Connecting to {}", self.socket_path);

        loop {
            // Wait for a command OR check connection status periodically?
            // For simplicity, we'll ensure connection *before* processing each command.
            tokio::select! {
                Some((command, response_tx)) = self.receiver.recv() => {
                    debug!("Actor received command: {:?}", command);
                    let result = self.process_command(command).await;
                    if let Err(e) = response_tx.send(result) {
                        error!("Failed to send response back to caller: {:?}", e);
                    }
                }
                else => {
                    info!("Command channel closed. HelperClientActor shutting down.");
                    break; // Exit loop if the sender side (HelperClient) is dropped
                }
            }
        }
    }

    /// Ensures a valid connection exists, attempting to reconnect if necessary.
    /// Returns mutable references to the reader and writer if successful.
    async fn ensure_connection(&mut self) -> HelperClientResult<(&mut BufReader<OwnedReadHalf>, &mut BufWriter<OwnedWriteHalf>)> {
        if self.writer.is_none() || self.reader.is_none() {
            debug!("No active connection, attempting to connect...");
            self.connect_with_retry().await?;
        } else {
            // Optionally add a cheap "is alive" check here if needed,
            // e.g., check stream.peer_addr(), although read/write errors
            // are the more definitive way to detect breakage.
        }

        // We need to return mutable refs, which requires careful handling
        // This unwrap is safe because connect_with_retry ensures they are Some
        Ok((self.reader.as_mut().unwrap(), self.writer.as_mut().unwrap()))
    }
    
    async fn connect_with_retry(&mut self) -> HelperClientResult<()> {
        let mut delay = RECONNECT_DELAY_MS;
        loop {
            match UnixStream::connect(&self.socket_path).await {
                Ok(stream) => {
                    // Split the stream for BufReader/BufWriter if necessary,
                    // but keeping them separate simplifies ownership for reconnection.
                    // Let's re-connect fully each time for simplicity here.
                    let (read_half, write_half) = stream.into_split();
                    self.reader = Some(BufReader::new(read_half));
                    self.writer = Some(BufWriter::new(write_half));
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to connect to helper socket: {}. Retrying in {}ms...", e, delay);
                    self.disconnect(); // Clear any partial state
                    sleep(Duration::from_millis(delay)).await;
                    delay = (delay * 2).min(MAX_RECONNECT_DELAY_MS); // Exponential backoff
                }
            }
        }
    }

    /// Clears the current connection state.
    fn disconnect(&mut self) {
        if self.writer.is_some() {
            info!("Disconnecting from helper service.");
        }
        self.writer = None;
        self.reader = None;
    }

    /// Processes a single command, handling connection and communication.
    async fn process_command(&mut self, command: ServerHelperCommand) -> HelperClientResult<()> {
        let request = ServerHelperRequest { command };
        let request_json = serde_json::to_string(&request)
            .map_err(HelperClientError::SerializationError)? + "\n"; // Add newline delimiter

        // Loop to handle potential write errors and trigger reconnect
        let response_line = loop {
            match self.ensure_connection().await {
                Ok((reader, writer)) => {
                    // Attempt to write
                    
                    match writer.write_all(request_json.as_bytes()).await{
                        Ok(_) => {
                            writer.flush().await.unwrap();
                        }
                        Err(e) => {
                            error!("Write error to helper socket: {}. Triggering reconnect.", e);
                            self.disconnect();
                            // Don't return error yet, loop will call ensure_connection again
                            continue;
                        }
                    }
                    // Attempt to read response
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            error!("Helper socket closed connection unexpectedly during read.");
                            self.disconnect();
                            return Err(HelperClientError::InternalError("Connection closed by peer".to_string()));
                        }
                        Ok(_) => {
                            // Successfully read a line, break the loop
                            break line;
                        }
                        Err(e) => {
                            error!("Read error from helper socket: {}. Triggering reconnect.", e);
                            self.disconnect();
                            // Check if it's a recoverable error before returning; if not, return
                            // For now, assume most IO errors mean the connection is dead.
                            return Err(HelperClientError::IoError(e));
                        }
                    }
                }
                Err(e) => {
                    // ensure_connection failed after retries
                    error!("Failed to establish connection after retries: {}", e);
                    return Err(e); // Propagate connection failure
                }
            }
        }; // End loop


        // Deserialize response
        let response: ServerHelperResponse = serde_json::from_str(response_line.trim())
            .map_err(|e| {
                error!("Failed to deserialize response: {}. Raw: '{}'", e, response_line.trim());
                HelperClientError::DeserializationError(e)
            })?;
        

        // Check response status
        match response.status {
            ServerHelperResponseStatus::Success => Ok(()),
            ServerHelperResponseStatus::Error(msg) => Err(HelperClientError::ServiceError(msg)),
        }
    }
}

/// Spawns the actor task and returns the client handle.
pub fn start_helper_client(socket_path: String) -> HelperClient {
    let (command_tx, command_rx) = mpsc::channel(500); // Buffer size 100
    let actor = HelperClientActor::new(socket_path, command_rx);

    tokio::spawn(actor.run()); // Spawn the actor task

    HelperClient::new(command_tx) // Return the client handle
}



