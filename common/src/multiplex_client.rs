
use dashmap::DashMap;
use deadpool::managed;
use deadpool::managed::{Metrics, Pool, RecycleError, RecycleResult};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use secrecy:: SecretString;

use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
#[cfg(feature = "multiplex-client-tcp")]
use secrecy::ExposeSecret;
#[cfg(feature = "multiplex-client-tcp")]
use tokio::net::{TcpStream};
#[cfg(feature = "multiplex-client-unix")]
use tokio::net::{UnixStream};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing::log::warn;
use tracing::{debug, error, info};
use crate::multiplex_protocol::{ActionTrait, GenericRequest, GenericResponse, ResponseTrait};
use crate::PING_PONG_ID;



type ResponseSender<Response> = oneshot::Sender<Result<GenericResponse<Response>, ClientError>>;
type RequestMap<Response> = DashMap<u64, ResponseSender<Response>>;
type CommandSender<Action, Response> =
mpsc::Sender<(GenericRequest<Action>, ResponseSender<Response>)>;
type CommandReceiver<Action, Response> =
mpsc::Receiver<(GenericRequest<Action>, ResponseSender<Response>)>;

pub struct ConnectionHandler<Action: ActionTrait, Response: ResponseTrait> {
    tx: CommandSender<Action, Response>,
    _action_marker: PhantomData<Action>,
    _response_marker: PhantomData<Response>,
}

impl<Action: ActionTrait, Response: ResponseTrait> ConnectionHandler<Action, Response> {


    #[cfg(feature = "multiplex-client-unix")]
    /// Establishes a connection and spawns background read/write tasks.
    pub async fn new(addr: String, token:Option<SecretString>) -> Result<Self, ConnectionError> {
        debug!("Attempting to connect to {}", addr);
        let stream = UnixStream::connect(&addr)
            .await
            .map_err(|e| ConnectionError::Io(e.to_string()))?; // Map connection error
        info!("Successfully connected to {}", addr);
        let (rd, wr) = tokio::io::split(stream);

        let (tx, rx): (
            CommandSender<Action, Response>,
            CommandReceiver<Action, Response>,
        ) = mpsc::channel(100);
        let request_map = Arc::new(RequestMap::<Response>::new());

        // Spawn reader task
        let reader_map_clone = Arc::clone(&request_map);
        tokio::spawn(Self::read_task(rd, reader_map_clone));

        // Spawn writer task
        tokio::spawn(Self::write_task(token, wr, rx, request_map));

        Ok(ConnectionHandler {
            tx,
            _action_marker: PhantomData,
            _response_marker: PhantomData,
        })
    }

    #[cfg(feature = "multiplex-client-unix")]
    /// Task to read responses from the socket and notify waiting callers.
    async fn read_task(mut rd: ReadHalf<UnixStream>, request_map: Arc<RequestMap<Response>>) {
        info!("Reader task started");
        let mut error_occurred = false;
        loop {
            // 1. Read Length
            let len = match rd.read_u32().await {
                Ok(0) => {
                    info!("Reader: Connection closed (read 0 length)");
                    break;
                }
                Ok(len) => len,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!("Reader: Connection closed cleanly (EOF reading length)");
                    } else {
                        error!("Reader: Failed to read length: {}", e);
                        error_occurred = true; // Mark error for cleanup notification
                    }
                    break;
                }
            };

            // Basic sanity check
            if len == 0 || len > 10 * 1024 * 1024 {
                // e.g., 10MB limit
                error!("Reader: Received invalid length: {}", len);
                error_occurred = true;
                break;
            }

            // 2. Read Payload
            let mut buffer = vec![0u8; len as usize];
            if let Err(e) = rd.read_exact(&mut buffer).await {
                error!("Reader: Failed to read payload: {}", e);
                error_occurred = true;
                break;
            }

            // 3. Deserialize Generic Response
            let response: GenericResponse<Response> = match serde_json::from_slice(&buffer) {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Reader: Failed to deserialize response: {}", e);
                    error_occurred = true;
                    // Don't break necessarily, maybe log and try next message?
                    // For now, breaking on deserialize error seems safer.
                    break;
                }
            };

            debug!("Reader: Received Resp ID: {}", response.id);

            // Handle Ping/Pong specially for recycling checks
            if response.id == PING_PONG_ID {
                // We still need to notify the sender waiting in `recycle`
                if let Some((_id, sender)) = request_map.remove(&response.id) {
                    debug!("Reader: Found match for Ping Resp ID: {}", response.id);
                    // Check if it's actually a Pong
                    if response.action_response == Response::get_pong() {
                        info!("Reader: Received valid Pong for ID {}", PING_PONG_ID);
                        // Send Ok(response) back to the recycler
                        let _ = sender.send(Ok(response));
                    } else {
                        warn!(
                            "Reader: Received non-Pong response for Ping ID {}: {:?}",
                            PING_PONG_ID, response.action_response
                        );
                        // Send error back to the recycler
                        let _ =
                            sender.send(Err(ClientError::Connection(ConnectionError::InvalidPong)));
                    }
                } else {
                    // This shouldn't happen if recycle logic is correct
                    warn!(
                        "Reader: Received Pong response for ID {} but no sender waiting?",
                        PING_PONG_ID
                    );
                }
                continue; // Continue reading after handling pong
            }

            // 4. Match Regular Response to Request
            if let Some((_id, sender)) = request_map.remove(&response.id) {
                debug!("Reader: Found match for Resp ID: {}", response.id);

                // Check if the *inner* response indicates an error
                let result = match response.action_response.get_error() {
                    Some(err_msg) => Err(ClientError::Server(err_msg)),
                    None => Ok(response), // Pass the full GenericResponse<Response>
                };

                if sender.send(result).is_err() {
                    // Caller might have timed out or dropped the request
                    debug!(
                        "Reader: Failed to send response back to caller (channel closed for ID: {})",
                        _id
                    );
                }
            } else {
                warn!("Reader: Received unexpected response ID: {}", response.id);
                // Potentially malicious or bug? Log and discard.
            }
        }

        // --- Cleanup ---
        info!("Reader task finished. Cleaning up outstanding requests.");
        let err_type = if error_occurred {
            ClientError::Connection(ConnectionError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, "Reader task failed unexpectedly")
                    .to_string(),
            ))
        } else {
            ClientError::Connection(ConnectionError::Closed)
        };

        // Notify all remaining pending requests
        let keys_to_notify: Vec<u64> = request_map.iter().map(|entry| *entry.key()).collect();

        for key in keys_to_notify {
            // Attempt to remove the entry. It might already be gone if processed right before loop exit.
            if let Some((id, sender)) = request_map.remove(&key) {
                info!(
                    "Reader: Notifying request ID {} of connection closure/error",
                    id
                );
                let _ = sender.send(Err(err_type.clone()));
            }
        }
        info!("Reader task cleanup complete.");
    }

    #[cfg(feature = "multiplex-client-unix")]
    /// Task to write requests received from the client handle to the socket.
    async fn write_task(
        _token:Option<SecretString>,
        mut wr: WriteHalf<UnixStream>,
        mut rx: CommandReceiver<Action, Response>,
        request_map: Arc<RequestMap<Response>>, // Needed to store sender
    ) {
        info!("Writer task started");
        while let Some((request, sender)) = rx.recv().await {
            let request_id = request.id;

            // Store sender *before* attempting to write
            if request_map.insert(request_id, sender).is_some() {
                warn!("Writer: Duplicate request ID {} encountered!", request_id);
                // Overwriting the previous sender. This indicates a bug in ID generation
                // or extremely rapid requests. Let's log and continue for now.
            }
            debug!("Writer: Stored sender for Req ID: {}", request_id);

            // 1. Serialize Generic Request
            let request_bytes = match serde_json::to_vec(&request) {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!(
                        "Writer: Failed to serialize request ID {}: {}",
                        request_id, e
                    );
                    if let Some((_id, sender)) = request_map.remove(&request_id) {
                        let _ = sender.send(Err(ClientError::Connection(
                            ConnectionError::SerdeError(e.to_string()),
                        )));
                    }
                    continue; // Skip to next request
                }
            };

            // 2. Write Length Prefix
            if let Err(e) = wr.write_u32(request_bytes.len() as u32).await {
                error!(
                    "Writer: Failed to write length for Req ID {}: {}",
                    request_id, e
                );
                if let Some((_id, sender)) = request_map.remove(&request_id) {
                    let _ = sender.send(Err(ClientError::Connection(e.into())));
                }
                break; // Connection likely broken
            }

            // 3. Write Payload
            if let Err(e) = wr.write_all(&request_bytes).await {
                error!(
                    "Writer: Failed to write payload for Req ID {}: {}",
                    request_id, e
                );
                if let Some((_id, sender)) = request_map.remove(&request_id) {
                    let _ = sender.send(Err(ClientError::Connection(e.into())));
                }
                break; // Connection likely broken
            }
            debug!("Writer: Sent Req ID: {}", request_id);
        }
        info!("Writer task finished (channel closed or write error).");
        // Attempt graceful shutdown of write half
        let _ = wr.shutdown().await;
    }

    #[cfg(feature = "multiplex-client-tcp")]
    /// Establishes a connection and spawns background read/write tasks.
    pub async fn new(addr: String, token:Option<SecretString>) -> Result<Self, ConnectionError> {
        debug!("Attempting to connect to {}", addr);
        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ConnectionError::Io(e.to_string()))?; // Map connection error
        info!("Successfully connected to {}", addr);
        let (rd, wr) = tokio::io::split(stream);

        let (tx, rx): (
            CommandSender<Action, Response>,
            CommandReceiver<Action, Response>,
        ) = mpsc::channel(100);
        let request_map = Arc::new(RequestMap::<Response>::new());

        // Spawn reader task
        let reader_map_clone = Arc::clone(&request_map);
        tokio::spawn(Self::read_task(rd, reader_map_clone));

        // Spawn writer task
        tokio::spawn(Self::write_task(token, wr, rx, request_map));

        Ok(ConnectionHandler {
            tx,
            _action_marker: PhantomData,
            _response_marker: PhantomData,
        })
    }

    #[cfg(feature = "multiplex-client-tcp")]
    /// Task to read responses from the socket and notify waiting callers.
    async fn read_task(mut rd: ReadHalf<TcpStream>, request_map: Arc<RequestMap<Response>>) {
        info!("Reader task started");
        let mut error_occurred = false;
        loop {
            // 1. Read Length
            let len = match rd.read_u32().await {
                Ok(0) => {
                    info!("Reader: Connection closed (read 0 length)");
                    break;
                }
                Ok(len) => len,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!("Reader: Connection closed cleanly (EOF reading length)");
                    } else {
                        error!("Reader: Failed to read length: {}", e);
                        error_occurred = true; // Mark error for cleanup notification
                    }
                    break;
                }
            };

            // Basic sanity check
            if len == 0 || len > 10 * 1024 * 1024 {
                // e.g., 10MB limit
                error!("Reader: Received invalid length: {}", len);
                error_occurred = true;
                break;
            }

            // 2. Read Payload
            let mut buffer = vec![0u8; len as usize];
            if let Err(e) = rd.read_exact(&mut buffer).await {
                error!("Reader: Failed to read payload: {}", e);
                error_occurred = true;
                break;
            }

            // 3. Deserialize Generic Response
            let response: GenericResponse<Response> = match serde_json::from_slice(&buffer) {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Reader: Failed to deserialize response: {}", e);
                    error_occurred = true;
                    // Don't break necessarily, maybe log and try next message?
                    // For now, breaking on deserialize error seems safer.
                    break;
                }
            };

            debug!("Reader: Received Resp ID: {}", response.id);

            // Handle Ping/Pong specially for recycling checks
            if response.id == PING_PONG_ID {
                // We still need to notify the sender waiting in `recycle`
                if let Some((_id, sender)) = request_map.remove(&response.id) {
                    debug!("Reader: Found match for Ping Resp ID: {}", response.id);
                    // Check if it's actually a Pong
                    if response.action_response == Response::get_pong() {
                        info!("Reader: Received valid Pong for ID {}", PING_PONG_ID);
                        // Send Ok(response) back to the recycler
                        let _ = sender.send(Ok(response));
                    } else {
                        warn!(
                            "Reader: Received non-Pong response for Ping ID {}: {:?}",
                            PING_PONG_ID, response.action_response
                        );
                        // Send error back to the recycler
                        let _ =
                            sender.send(Err(ClientError::Connection(ConnectionError::InvalidPong)));
                    }
                } else {
                    // This shouldn't happen if recycle logic is correct
                    warn!(
                        "Reader: Received Pong response for ID {} but no sender waiting?",
                        PING_PONG_ID
                    );
                }
                continue; // Continue reading after handling pong
            }

            // 4. Match Regular Response to Request
            if let Some((_id, sender)) = request_map.remove(&response.id) {
                debug!("Reader: Found match for Resp ID: {}", response.id);

                // Check if the *inner* response indicates an error
                let result = match response.action_response.get_error() {
                    Some(err_msg) => Err(ClientError::Server(err_msg)),
                    None => Ok(response), // Pass the full GenericResponse<Response>
                };

                if sender.send(result).is_err() {
                    // Caller might have timed out or dropped the request
                    debug!(
                        "Reader: Failed to send response back to caller (channel closed for ID: {})",
                        _id
                    );
                }
            } else {
                warn!("Reader: Received unexpected response ID: {}", response.id);
                // Potentially malicious or bug? Log and discard.
            }
        }

        // --- Cleanup ---
        info!("Reader task finished. Cleaning up outstanding requests.");
        let err_type = if error_occurred {
            ClientError::Connection(ConnectionError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, "Reader task failed unexpectedly")
                    .to_string(),
            ))
        } else {
            ClientError::Connection(ConnectionError::Closed)
        };

        // Notify all remaining pending requests
        let keys_to_notify: Vec<u64> = request_map.iter().map(|entry| *entry.key()).collect();

        for key in keys_to_notify {
            // Attempt to remove the entry. It might already be gone if processed right before loop exit.
            if let Some((id, sender)) = request_map.remove(&key) {
                info!(
                    "Reader: Notifying request ID {} of connection closure/error",
                    id
                );
                let _ = sender.send(Err(err_type.clone()));
            }
        }
        info!("Reader task cleanup complete.");
    }

    #[cfg(feature = "multiplex-client-tcp")]
    /// Task to write requests received from the client handle to the socket.
    async fn write_task(
        token:Option<SecretString>,
        mut wr: WriteHalf<TcpStream>,
        mut rx: CommandReceiver<Action, Response>,
        request_map: Arc<RequestMap<Response>>, // Needed to store sender
    ) {
        info!("Writer task started");
        let request_bytes=  serde_json::to_vec(&GenericRequest::<Action>::get_auth(token.expect("Token Should be present for tcp auth").expose_secret().to_string())).expect("Failed to serialize auth request");
        if let Err(e) = wr.write_u32(request_bytes.len() as u32).await {
            error!(
                "Writer: Failed to write length for Req ID {}: {}",
                PING_PONG_ID, e
            );
            return;
        }
        if let Err(e) = wr.write_all(&request_bytes).await {
            error!(
                "Writer: Failed to write payload for Req ID {}: {}",
                PING_PONG_ID, e
            );
            return;
        }

        while let Some((request, sender)) = rx.recv().await {
            let request_id = request.id;

            // Store sender *before* attempting to write
            if request_map.insert(request_id, sender).is_some() {
                warn!("Writer: Duplicate request ID {} encountered!", request_id);
                // Overwriting the previous sender. This indicates a bug in ID generation
                // or extremely rapid requests. Let's log and continue for now.
            }
            debug!("Writer: Stored sender for Req ID: {}", request_id);

            // 1. Serialize Generic Request
            let request_bytes = match serde_json::to_vec(&request) {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!(
                        "Writer: Failed to serialize request ID {}: {}",
                        request_id, e
                    );
                    if let Some((_id, sender)) = request_map.remove(&request_id) {
                        let _ = sender.send(Err(ClientError::Connection(
                            ConnectionError::SerdeError(e.to_string()),
                        )));
                    }
                    continue; // Skip to next request
                }
            };

            // 2. Write Length Prefix
            if let Err(e) = wr.write_u32(request_bytes.len() as u32).await {
                error!(
                    "Writer: Failed to write length for Req ID {}: {}",
                    request_id, e
                );
                if let Some((_id, sender)) = request_map.remove(&request_id) {
                    let _ = sender.send(Err(ClientError::Connection(e.into())));
                }
                break; // Connection likely broken
            }

            // 3. Write Payload
            if let Err(e) = wr.write_all(&request_bytes).await {
                error!(
                    "Writer: Failed to write payload for Req ID {}: {}",
                    request_id, e
                );
                if let Some((_id, sender)) = request_map.remove(&request_id) {
                    let _ = sender.send(Err(ClientError::Connection(e.into())));
                }
                break; // Connection likely broken
            }
            debug!("Writer: Sent Req ID: {}", request_id);
        }
        info!("Writer task finished (channel closed or write error).");
        // Attempt graceful shutdown of write half
        let _ = wr.shutdown().await;
    }




    /// Internal method used by the connection manager for recycling.
    async fn send_ping_internal(&self) -> Result<GenericResponse<Response>, ClientError> {
        // Use the ActionTrait's HasPing constraint
        let request = GenericRequest::<Action>::get_ping();
        let (tx_resp, rx_resp) = oneshot::channel();

        debug!("Sending Ping Req ID: {}", request.id);
        // Send request to the writer task via the MPSC channel
        if self.tx.send((request, tx_resp)).await.is_err() {
            error!("Failed to send Ping request to writer task (channel closed)");
            return Err(ClientError::Connection(ConnectionError::Closed));
        }

        // Wait for the response from the reader task
        match rx_resp.await {
            Ok(result) => {
                debug!("Received result for Ping Req ID: {}", PING_PONG_ID);
                result // Propagate the Result<GenericResponse<Response>, ClientError>
            }
            Err(_) => {
                error!(
                    "Response channel closed prematurely for Ping Req ID: {}",
                    PING_PONG_ID
                );
                Err(ClientError::Connection(
                    ConnectionError::ResponseChannelClosed,
                ))
            }
        }
    }
}

struct ConnectionManager<Action: ActionTrait, Response: ResponseTrait> {
    addr: String,
    token: Option<SecretString>,
    _marker: PhantomData<(Action, Response)>, // Use PhantomData for unused generics
}

impl<Action: ActionTrait, Response: ResponseTrait> managed::Manager
for ConnectionManager<Action, Response>
{
    type Type = ConnectionHandler<Action, Response>; // The managed type
    type Error = ConnectionError; // The error type for connection issues

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        // Creates a new connection handler
        ConnectionHandler::new(self.addr.clone(), self.token.clone()).await
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _metrics: &Metrics,
    ) -> RecycleResult<Self::Error> {
        // Recycles an existing connection handler by sending a Ping
        debug!("Recycling connection...");
        let recycle_timeout = Duration::from_millis(500); // Increased timeout slightly

        match timeout(recycle_timeout, conn.send_ping_internal()).await {
            // Ping timed out
            Err(_) => {
                warn!("Recycle failed: Ping timed out for {}", self.addr);
                Err(RecycleError::Backend(ConnectionError::PingTimeout))
            }
            // Got a result within timeout
            Ok(ping_result) => match ping_result {
                // Successfully received a response for the Ping
                Ok(response) => {
                    // Use the ResponseTrait's HasPong constraint to check
                    if response.action_response == Response::get_pong() {
                        debug!("Recycle successful (Pong received) for {}", self.addr);
                        Ok(()) // Connection is healthy
                    } else {
                        warn!(
                            "Recycle failed: Invalid Pong received from {}: {:?}",
                            self.addr, response.action_response
                        );
                        Err(RecycleError::Backend(ConnectionError::InvalidPong))
                    }
                }
                // send_ping_internal failed (ConnectionError or other ClientError variants)
                Err(ClientError::Connection(e)) => {
                    warn!(
                        "Recycle failed: Connection error during Ping to {}: {:?}",
                        self.addr, e
                    );
                    Err(RecycleError::Backend(e)) // Propagate the specific connection error
                }
                Err(ClientError::Server(e)) => {
                    // This shouldn't happen for a Ping/Pong
                    error!(
                        "Recycle failed: Server returned error for Ping to {}: {}",
                        self.addr, e
                    );
                    Err(RecycleError::Message(std::borrow::Cow::from(format!(
                        "Server error on ping: {}",
                        e
                    ))))
                }
                Err(e) => {
                    // Catch-all for other ClientError variants
                    error!(
                        "Recycle failed: Unexpected client error during Ping to {}: {:?}",
                        self.addr, e
                    );
                    Err(RecycleError::Message(std::borrow::Cow::from(format!(
                        "Unexpected Ping Error: {}",
                        e
                    ))))
                }
            },
        }
    }
}

type GenericPool<Action, Response> = Pool<ConnectionManager<Action, Response>>;

#[derive(Clone)] // Add Clone
pub struct MultiplexClient<Action: ActionTrait, Response: ResponseTrait> {
    pool: GenericPool<Action, Response>,
    next_request_id: Arc<AtomicU64>,
    _marker: PhantomData<fn() -> (Action, Response)>, // Use function pointer marker
}

impl<Action: ActionTrait, Response: ResponseTrait> MultiplexClient<Action, Response> {
    /// Creates a new client with a connection pool.
    pub fn new(addr: String, token:Option<SecretString>, max_size: usize) -> Result<Self, ConnectionError> {
        let manager = ConnectionManager {
            addr,
            token,
            _marker: PhantomData,
        };
        let pool = Pool::builder(manager)
            .max_size(max_size)
            .build()
            .map_err(|e| ConnectionError::PoolError(e.to_string()))?;

        Ok(MultiplexClient {
            pool,
            next_request_id: Arc::new(AtomicU64::new(1)), // Start IDs from 1
            _marker: PhantomData,
        })
    }

    /// Sends an action and waits for the corresponding response.
    pub async fn send(&self, action: Action) -> Result<Response, ClientError> {
        // 1. Get a connection handler from the pool
        let handler = self.pool.get().await?;
        debug!("Got connection handler from pool");

        // 2. Generate unique request ID (ensure it's not PING_PONG_ID)
        let request_id = loop {
            let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
            // Handle potential wrap-around to 0, although very unlikely with u64
            if id != PING_PONG_ID {
                break id;
            } else {
                warn!("Generated PING_PONG_ID, incrementing again.");
                // Fetch-add again if we hit the reserved ID
                self.next_request_id.fetch_add(1, Ordering::Relaxed);
            }
        };

        // 3. Create Request wrapper and oneshot channel
        let request = GenericRequest {
            id: request_id,
            action,
        };
        let (tx_resp, rx_resp) = oneshot::channel();

        // 4. Send request to the handler's writer task
        // The handler object obtained from the pool needs to expose its 'tx' channel,
        // or have a method to send the request. Let's assume ConnectionHandler's tx is accessible
        // (or add a helper method to ConnectionHandler if tx is private).
        if handler.tx.send((request, tx_resp)).await.is_err() {
            error!(
                "Client: Failed to send request ID {} to handler task (channel closed)",
                request_id
            );
            // Connection is likely dead. Deadpool will handle eviction on recycle failure.
            return Err(ClientError::Connection(ConnectionError::Closed));
        }
        debug!("Client: Sent request ID {} to handler channel", request_id);

        // 5. Wait for the response with a timeout
        let request_timeout = Duration::from_secs(10); // Configurable?
        match timeout(request_timeout, rx_resp).await {
            // Request timed out
            Err(_) => {
                error!("Client: Request ID {} timed out", request_id);
                // Note: The request might still be processed by the server.
                // The sender in the request_map on the handler side will eventually be dropped
                // or cleaned up when the reader task notices the closed connection.
                Err(ClientError::RequestTimeout)
            }
            // Received result from oneshot channel
            Ok(Ok(handler_result)) => {
                // handler_result is Result<GenericResponse<Response>, ClientError>
                debug!("Client: Received result for Req ID: {}", request_id);
                // Extract the inner action_response if Ok, otherwise propagate ClientError
                handler_result.map(|resp| resp.action_response)
            }
            Ok(Err(_oneshot_err)) => {
                // The oneshot channel was closed before a response was sent.
                // This usually means the ConnectionHandler task panicked or exited.
                error!(
                    "Client: Response channel closed prematurely for Req ID: {}",
                    request_id
                );
                Err(ClientError::Connection(
                    ConnectionError::ResponseChannelClosed,
                ))
            }
        }
        // ConnectionHandler (pooled object) is dropped here, returned to pool.
    }
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ConnectionError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Connection closed by peer")]
    Closed,
    #[error("Response channel closed")]
    ResponseChannelClosed,
    #[error("Request map interaction failed")]
    RequestMapError,
    #[error("Deadpool Error: {0}")]
    PoolError(String), // Wrap deadpool errors if needed
    #[error("Serialization error: {0}")]
    SerdeError(String),
    #[error("Ping timed out")]
    PingTimeout,
    #[error("Invalid Pong received")]
    InvalidPong,
    #[error("Connection Unhealthy (Recycle Failed)")]
    Unhealthy, // General recycle failure
}

impl From<serde_json::Error> for ConnectionError {
    fn from(err: serde_json::Error) -> Self {
        ConnectionError::SerdeError(err.to_string())
    }
}

impl From<std::io::Error> for ConnectionError {
    fn from(err: std::io::Error) -> Self {
        ConnectionError::Io(err.to_string())
    }
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ClientError {
    #[error("Connection Error: {0}")]
    Connection(#[from] ConnectionError), // Errors from connection layer
    #[error("Pool Error: {0}")]
    Pool(String),
    #[error("Request timed out")]
    RequestTimeout, // Specific timeout error for send
    #[error("Server returned error: {0}")]
    Server(String),
}

impl From<deadpool::managed::PoolError<ConnectionError>> for ClientError {
    fn from(err: deadpool::managed::PoolError<ConnectionError>) -> Self {
        ClientError::Pool(err.to_string())
    }
}
