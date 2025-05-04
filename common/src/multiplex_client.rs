use dashmap::DashMap;
use deadpool::managed;
use deadpool::managed::{Metrics, Pool, RecycleError, RecycleResult};
use secrecy::SecretString;
use bitcode::{Decode, Encode};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::multiplex_protocol::{ActionTrait, GenericRequest, GenericResponse, ResponseTrait};
use crate::PING_PONG_ID;
#[cfg(feature = "multiplex-client-tcp")]
use secrecy::ExposeSecret;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
#[cfg(feature = "multiplex-client-tcp")]
use tokio::net::TcpStream;
#[cfg(feature = "multiplex-client-unix")]
use tokio::net::UnixStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing::log::warn;
use tracing::{debug, error, info};

type ResponseSender<Response> = oneshot::Sender<Result<GenericResponse<Response>, ClientError>>;
type RequestMap<Response> = DashMap<u64, ResponseSender<Response>>;
type CommandSender<Action, Response> =
    mpsc::Sender<(GenericRequest<Action>, ResponseSender<Response>)>;
type CommandReceiver<Action, Response> =
    mpsc::Receiver<(GenericRequest<Action>, ResponseSender<Response>)>;


async fn read_task<R, Response>(mut rd: R, request_map: Arc<RequestMap<Response>>)
where
    R: AsyncRead + Unpin + Send + 'static,
    Response: ResponseTrait,
{
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
        if len > 10 * 1024 * 1024 {
            // e.g., 10MB limit
            error!("Reader: Received invalid length: {}", len);
            error_occurred = true;
            break;
        }
        if len == 0 {
            info!("Reader: Received zero length message body. Assuming EOF/closed.");
            break; // Treat length 0 as connection closed after length read
        }


        // 2. Read Payload
        let mut buffer = vec![0u8; len as usize];
        if let Err(e) = rd.read_exact(&mut buffer).await {
            error!("Reader: Failed to read payload: {}", e);
            error_occurred = true;
            break;
        }

        // 3. Deserialize Generic Response
        let response: GenericResponse<Response> = match bitcode::decode(&buffer) {
            Ok(resp) => resp,
            Err(e) => {
                error!("Reader: Failed to deserialize response: {}", e);
                error_occurred = true;
                // Breaking on deserialize error seems safer.
                break;
            }
        };

        debug!("Reader: Received Resp ID: {}", response.id);

        // Handle Ping/Pong specially for recycling checks
        if response.id == PING_PONG_ID {
            if let Some((_id, sender)) = request_map.remove(&response.id) {
                debug!("Reader: Found match for Ping Resp ID: {}", response.id);
                if response.action_response == Response::get_pong() {
                    debug!("Reader: Received valid Pong for ID {}", PING_PONG_ID);
                    let _ = sender.send(Ok(response)); // Send Ok(Pong response)
                } else {
                    warn!(
                        "Reader: Received non-Pong response for Ping ID {}: {:?}",
                        PING_PONG_ID, response.action_response
                    );
                    let _ = sender.send(Err(ClientError::Connection(ConnectionError::InvalidPong)));
                }
            } else {
                warn!( // Changed to warn!
                    "Reader: Received Pong response for ID {PING_PONG_ID} but no sender waiting?"
                );
            }
        } else {
            // Handle regular responses
            if let Some((_id, sender)) = request_map.remove(&response.id) {
                debug!("Reader: Found match for Resp ID: {}", response.id);
                let result = match response.action_response.get_error() {
                    Some(err_msg) => Err(ClientError::Server(err_msg)),
                    None => Ok(response), // Pass the full GenericResponse
                };
                if sender.send(result).is_err() {
                    warn!( // Changed to warn!
                        "Reader: Failed to send response back to caller (channel closed for ID: {_id})",
                    );
                }
            } else {
                warn!( // Changed to warn!
                    "Reader: Received unexpected response ID: {}", response.id
                );
            }
        }
    }

    info!("Reader task finished. Cleaning up outstanding requests.");
    let err_type = if error_occurred {
        ClientError::Connection(ConnectionError::Io(
            std::io::Error::other(
                "Reader task failed unexpectedly",
            )
                .to_string(),
        ))
    } else {
        ClientError::Connection(ConnectionError::Closed)
    };

    // Notify all remaining waiters about the connection failure/closure
    let keys_to_notify: Vec<u64> = request_map.iter().map(|entry| *entry.key()).collect();
    for key in keys_to_notify {
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


async fn write_task<W, Action, Response>(
    mut wr: W,
    mut rx: CommandReceiver<Action, Response>,
    request_map: Arc<RequestMap<Response>>, // Needed to store sender
) where
    W: AsyncWrite + Unpin + Send + 'static,
    Action: ActionTrait,
    Response: ResponseTrait,
{
    info!("Writer task started");
    while let Some((request, sender)) = rx.recv().await {
        let request_id = request.id;

        // Store sender *before* attempting to write
        if request_map.insert(request_id, sender).is_some() {
            warn!("Writer: Duplicate request ID {request_id} encountered!");
        }
        debug!("Writer: Stored sender for Req ID: {request_id}");

        // 1. Serialize Generic Request
        let request_bytes = bitcode::encode(&request);
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


pub struct ConnectionHandler<Action: ActionTrait, Response: ResponseTrait> {
    tx: CommandSender<Action, Response>,
    _action_marker: PhantomData<Action>,
    _response_marker: PhantomData<Response>,
}

impl<Action: ActionTrait, Response: ResponseTrait> ConnectionHandler<Action, Response> {
    #[cfg(feature = "multiplex-client-unix")]
    /// Establishes a Unix socket connection and spawns background read/write tasks.
    pub async fn new_unix(addr: String) -> Result<Self, ConnectionError> {
        info!("Connecting to Unix socket: {}", addr);
        let stream = UnixStream::connect(&addr)
            .await
            .map_err(|e| ConnectionError::Io(e.to_string()))?;
        info!("Successfully connected to Unix socket: {}", addr);
        let (rd, wr): (ReadHalf<UnixStream>, WriteHalf<UnixStream>) = tokio::io::split(stream);

        Self::spawn_tasks(rd, wr)
    }

    #[cfg(feature = "multiplex-client-tcp")]
    /// Establishes a TCP connection, performs auth, and spawns background read/write tasks.
    pub async fn new_tcp(addr: String, token: Option<SecretString>) -> Result<Self, ConnectionError> {
        info!("Connecting to TCP socket: {}", addr);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ConnectionError::Io(e.to_string()))?;
        info!("Successfully connected to TCP socket: {}", addr);

        // --- Perform Authentication ---
        let auth_token = token.ok_or(ConnectionError::AuthRequired)?;
        let auth_request = GenericRequest::<Action>::get_auth(auth_token.expose_secret().to_string())
            .ok_or(ConnectionError::AuthNotSupported)?; // Assuming get_auth returns Option

        let auth_bytes = bitcode::encode(&auth_request);

        stream.write_u32(auth_bytes.len() as u32).await?;
        stream.write_all(&auth_bytes).await?;
        info!("Sent authentication token to {}", addr);
        // Note: This assumes the server doesn't send an immediate auth confirmation response.
        // If it does, you'd need to read it here *before* splitting the stream.

        let (rd, wr): (ReadHalf<TcpStream>, WriteHalf<TcpStream>) = tokio::io::split(stream);

        Self::spawn_tasks(rd, wr)
    }

    fn spawn_tasks<R, W>(rd: R, wr: W) -> Result<Self, ConnectionError>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, rx): (
            CommandSender<Action, Response>,
            CommandReceiver<Action, Response>,
        ) = mpsc::channel(100);

        let request_map = Arc::new(RequestMap::<Response>::new());

        // Spawn generic reader task
        let reader_map_clone = Arc::clone(&request_map);
        tokio::spawn(read_task(rd, reader_map_clone));

        // Spawn generic writer task
        tokio::spawn(write_task(wr, rx, request_map));

        Ok(ConnectionHandler {
            tx,
            _action_marker: PhantomData,
            _response_marker: PhantomData,
        })
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
    _token: Option<SecretString>,
    _marker: PhantomData<(Action, Response)>, // Use PhantomData for unused generics
}

impl<Action: ActionTrait, Response: ResponseTrait> managed::Manager
    for ConnectionManager<Action, Response>
{
    type Type = ConnectionHandler<Action, Response>; // The managed type
    type Error = ConnectionError; // The error type for connection issues

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        // Creates a new connection handler
        info!("Creating new pooled connection to {}", self.addr);
        #[cfg(feature = "multiplex-client-unix")]
        {
            ConnectionHandler::new_unix(self.addr.clone()).await
        }
        #[cfg(feature = "multiplex-client-tcp")]
        {
            ConnectionHandler::new_tcp(self.addr.clone(), self._token.clone()).await
        }
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _metrics: &Metrics,
    ) -> RecycleResult<Self::Error> {
        debug!("Recycling connection to {}", self.addr);
        let recycle_timeout = Duration::from_millis(500);
        match timeout(recycle_timeout, conn.send_ping_internal()).await {
            Err(_) => {
                warn!("Recycle failed: Ping timed out for {}", self.addr);
                Err(RecycleError::Backend(ConnectionError::PingTimeout))
            }
            Ok(ping_result) => match ping_result {
                Ok(response) => {
                    if response.action_response == Response::get_pong() {
                        debug!("Recycle successful (Pong received) for {}", self.addr);
                        Ok(())
                    } else {
                        warn!(
                            "Recycle failed: Invalid Pong received from {}: {:?}",
                            self.addr, response.action_response
                        );
                        Err(RecycleError::Backend(ConnectionError::InvalidPong))
                    }
                }
                Err(ClientError::Connection(e)) => {
                    warn!(
                        "Recycle failed: Connection error during Ping to {}: {:?}",
                        self.addr, e
                    );
                    Err(RecycleError::Backend(e))
                }
                Err(ClientError::Server(e)) => {
                    // This shouldn't happen for a Ping/Pong
                    error!(
                        "Recycle failed: Server returned error for Ping to {}: {}",
                        self.addr, e
                    );
                    Err(RecycleError::Message(format!("Server error on ping: {e}").into()))
                }
                Err(e) => {
                    error!(
                        "Recycle failed: Unexpected client error during Ping to {}: {:?}",
                        self.addr, e
                    );
                    Err(RecycleError::Message(format!("Unexpected Ping Error: {e}").into()))
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
    pub fn new(
        addr: String,
        _token: Option<SecretString>,
        max_size: usize,
    ) -> Result<Self, ConnectionError> {

        #[cfg(all(feature = "multiplex-client-unix", feature = "multiplex-client-tcp"))]
        compile_error!("Features 'multiplex-client-unix' and 'multiplex-client-tcp' are mutually exclusive.");

        #[cfg(not(any(feature="multiplex-client-unix", feature="multiplex-client-tcp")))]
        compile_error!("Either 'multiplex-client-unix' or 'multiplex-client-tcp' feature must be enabled for MultiplexClient");

        #[cfg(feature = "multiplex-client-tcp")]
        if _token.is_none() {
            return Err(ConnectionError::AuthRequired);
        }
        let manager = ConnectionManager {
            addr,
            _token,
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
        let request_id = if action == Action::get_ping() {
            PING_PONG_ID
        } else {
            self.next_request_id.fetch_add(1, Ordering::Relaxed)
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

#[derive(Debug, Error, Clone, Encode, Decode)]
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
    BitcodeError(String),
    #[error("Ping timed out")]
    PingTimeout,
    #[error("Invalid Pong received")]
    InvalidPong,
    #[error("Connection Unhealthy (Recycle Failed)")]
    Unhealthy,
    #[error("Authentication required")]
    AuthRequired,
    #[error("Authentication not supported")]
    AuthNotSupported,
}

impl From<bitcode::Error> for ConnectionError {
    fn from(err: bitcode::Error) -> Self {
        ConnectionError::BitcodeError(err.to_string())
    }
}

impl From<std::io::Error> for ConnectionError {
    fn from(err: std::io::Error) -> Self {
        ConnectionError::Io(err.to_string())
    }
}

#[derive(Debug, Error, Clone, Encode, Decode)]
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
