// Suggested location: e.g., hivehost_website/src/proxy_client.rs
// Or potentially common crate if used more widely, but ensure dependencies match.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info};







#[derive(Debug, thiserror::Error, Clone, Serialize, Deserialize)]
pub enum TarpcClientError {
    #[error("Client not connected. Reconnection attempt might be in progress or will be triggered if not.")]
    NotConnected,
    #[error("Inner client application error: {0}")]
    ClientError(String), // Captures application-level errors returned by the RPC call
    #[error("Connection establishment error: {0}")]
    ConnectionError(String), // Changed to String to avoid making ProxyError generic over transport error
    #[error("RPC error: {0}")]
    RpcError(String),
}

impl From<tarpc::client::RpcError> for TarpcClientError {
    fn from(err: tarpc::client::RpcError) -> Self {
        TarpcClientError::RpcError(err.to_string())
    }
}

// Handle potential IO errors during connect specifically
impl From<std::io::Error> for TarpcClientError {
    fn from(err: std::io::Error) -> Self {
        TarpcClientError::ConnectionError(err.to_string())
    }
}


pub type TarpcClientResult<T> = Result<T, TarpcClientError>;


type Connector<T> = Box<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<T, TarpcClientError>> + Send>>
    + Send
    + Sync,
>;


pub struct TarpcClient<T: Clone + Send + Sync + 'static> {
    inner: Arc<Mutex<Option<T>>>,
    server_addr: String,
    connector: Arc<Connector<T>>,
}

impl<T: Clone + Send + Sync + 'static> Clone for TarpcClient<T> {
    fn clone(&self) -> Self {
        TarpcClient {
            inner: Arc::clone(&self.inner),
            server_addr: self.server_addr.clone(),
            connector: Arc::clone(&self.connector),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> fmt::Debug for TarpcClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyClient")
            .field("server_addr", &self.server_addr)
            .field("inner", &"Arc<Mutex<Option<...>>>") // Avoid showing T directly unless T: Debug
            .field("connector", &"Arc<Connector<...>>")
            .finish()
    }
}


impl<T: Clone + Send + Sync + 'static> TarpcClient<T> {
    pub fn new<F, Fut>(server_addr: String, connect_fn: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, TarpcClientError>> + Send + 'static,
    {
        let connector: Connector<T> = Box::new(move |addr| Box::pin(connect_fn(addr)));
        TarpcClient {
            inner: Arc::new(Mutex::new(None)),
            server_addr,
            connector: Arc::new(connector),
        }
    }

    async fn establish_connection(
        connector: Arc<Connector<T>>,
        server_addr: String,
    ) -> Result<T, TarpcClientError> {
        println!("Establishing connection to server at {server_addr}...", );
        connector(server_addr).await // Call the stored connector function
    }


    pub(crate) async fn get_or_connect_client(&self) -> Result<T, TarpcClientError> {
        let mut inner_guard = self.inner.lock().await;

        if let Some(client) = inner_guard.as_ref() {
            println!("Client already connected to {}.", self.server_addr);
            return Ok(client.clone());
        }
        
        info!("Client not connected, attempting synchronous connection to {}...", self.server_addr);
        match Self::establish_connection(Arc::clone(&self.connector), self.server_addr.clone()).await {
            Ok(new_client) => {
                println!("Successfully connected to {}.", self.server_addr);
                *inner_guard = Some(new_client.clone()); // Store the new client
                Ok(new_client)
            }
            Err(e) => {
                println!("Failed to connect synchronously to {}: {:?}", self.server_addr, e);
              
                Err(e) 
            }
        }
    }
    
    
    pub async fn connect(&self) -> Result<(), TarpcClientError> {
        let mut inner_guard = self.inner.lock().await; // Lock mutex

        if inner_guard.is_some() {
            println!("Explicit connect: Client is already connected to {}.", self.server_addr);
            return Ok(());
        }

        println!(
            "Explicit connect: Attempting connection to server at {}...",
            self.server_addr
        );

        match Self::establish_connection(Arc::clone(&self.connector), self.server_addr.clone()).await {
            Ok(client_instance) => {
                *inner_guard = Some(client_instance);
                println!("Explicit connect: Successfully connected to {}.", self.server_addr);
                Ok(())
            }
            Err(e) => {
                println!(
                    "Explicit connect: Failed to connect to {}: {:?}",
                    self.server_addr, e
                );
                Err(e)
            }
        }
    }


    
    
    pub async fn is_connected(&self) -> bool {
        self.inner.lock().await.is_some()
    }

    pub async fn disconnect(&self) {
        let mut inner_guard = self.inner.lock().await;
        if inner_guard.is_some() {
            *inner_guard = None;
            info!("Client disconnected from {}.", self.server_addr);
        } else {
            debug!("Client already disconnected from {}.", self.server_addr);
        }
    }
}



