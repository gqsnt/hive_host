use async_compression::tokio::bufread::BrotliEncoder;
use common::{Slug};
use common::{get_project_prod_path, ProjectSlugStr};
use dashmap::DashMap;
use deadpool_postgres::tokio_postgres::NoTls;
use deadpool_postgres::{tokio_postgres, Pool};
use http::header::SERVER;
use http::{header, HeaderValue, Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use quick_cache::sync::{Cache, DefaultLifecycle};
use quick_cache::{DefaultHashBuilder, Weighter};
use socket2::{Domain, SockAddr, Socket};
use std::convert::Infallible;
use std::io;
use std::net::{AddrParseError, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use secrecy::SecretString;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::{runtime, task};
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use walkdir::WalkDir;

pub mod handler;

pub static HOSTING_PREFIX: LazyLock<String> = LazyLock::new(|| {
     ".localhost:3002".to_string()
    
});

pub type HostingResult<T> = Result<T, HostingError>;

#[derive(Debug, Error)]
pub enum HostingError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("Http error: {0}")]
    Http(#[from] http::Error),
    #[error("Addr parse error: {0}")]
    AddrParseError(#[from] AddrParseError),
    #[error("DotEnv error: {0}")]
    DotEnv(#[from] dotenvy::Error),
    #[error("Tokio postgres error: {0}")]
    TokioPostgres(#[from] tokio_postgres::Error),
    #[error("Custom {0}")]
    Custom(String),
}

pub static CACHE: LazyLock<DashMap<ProjectSlugStr, ProjectCache>> = LazyLock::new(DashMap::new);

pub static TOKEN: LazyLock<String> =
    LazyLock::new(|| dotenvy::var("TOKEN_AUTH").expect("HOSTING_URL must be set"));

pub static DB: LazyLock<Pool> = LazyLock::new(|| {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some(dotenvy::var("DB_NAME").expect("DB_NAME must be set"));
    cfg.user = Some(dotenvy::var("DB_USER").expect("DB_USER must be set"));
    cfg.password = Some(dotenvy::var("DB_PASSWORD").expect("DB_PASSWORD must be set"));
    cfg.host = Some(dotenvy::var("DB_HOST").expect("DB_HOST must be set"));
    cfg.port = Some(
        dotenvy::var("DB_PORT")
            .expect("DB_PORT must be set")
            .parse()
            .unwrap(),
    );
    cfg.create_pool(None, NoTls).expect("Failed to create pool")
});

pub static CONTENT_ENCODING_BR: HeaderValue = HeaderValue::from_static("br");

pub static CACHE_HEADER: HeaderValue = HeaderValue::from_static("public, max-age=31536000");
pub static SERVER_HEADER: HeaderValue = HeaderValue::from_static("localhost");

#[derive(Clone)]
pub struct BodyWeighter;

impl Weighter<KeyType, BodyType> for BodyWeighter {
    fn weight(&self, _key: &KeyType, val: &BodyType) -> u64 {
        val.len().max(1) as u64
    }
}

pub type BodyType = Bytes;
pub type KeyType = String;

pub type FileCacheType =
    Cache<KeyType, BodyType, BodyWeighter, DefaultHashBuilder, DefaultLifecycle<KeyType, BodyType>>;

#[derive(Clone, Debug)]
pub struct ProjectCache {
    pub paths: Arc<DashMap<String, FileInfo>>,
    pub file_cache: Arc<FileCacheType>,
}


#[derive(Clone)]
pub struct AppState{
    pub server_auth: Arc<SecretString>,
    pub connected:Arc<RwLock<bool>>,
}



impl Default for ProjectCache {
    fn default() -> Self {
        Self {
            paths: Arc::new(DashMap::new()),
            file_cache: Arc::new(Cache::with_weighter(500, 20_000_000, BodyWeighter)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileInfo {
    pub mime_type: String,
    pub full_path: String,
}

pub async fn cache_project_path(project_slug: ProjectSlugStr) {
    info!("Serving/Caching Project: {:?}", project_slug);
    let project_prod_root_str = get_project_prod_path(&project_slug);

    CACHE.remove(&project_slug);
    let canonical_project_root = match tokio::fs::canonicalize(&project_prod_root_str).await {
        Ok(path) => path,
        Err(e) => {
            error!(
                "Failed to canonicalize project production root {:?}: {}",
                project_prod_root_str, e
            );
            // Cannot cache project if root is inaccessible or invalid
            return;
        }
    };
    
    let entry = CACHE.entry(project_slug).or_default();
    let paths = entry.paths.clone();
    
    
    let walker = WalkDir::new(&project_prod_root_str)
        .follow_links(false)
        .into_iter();
    for dir_entry_result in walker.filter_map(|r| r.ok()) {
        let entry_path = dir_entry_result.path();
        
        // Filter out dot files/directories early
        if entry_path
            .file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        // 3. Canonicalize the path of the entry
        let canonical_entry_path = match tokio::fs::canonicalize(entry_path).await {
            Ok(path) => path,
            Err(e) => {
                debug!("Failed to canonicalize path {:?}: {}", entry_path, e);
                continue; 
            }
        };

        // Check if it's a file *after* canonicalization
        let metadata = match tokio::fs::metadata(&canonical_entry_path).await {
            Ok(meta) => meta,
            Err(e) => {
                debug!("Failed to get metadata for canonical path {:?}: {}", canonical_entry_path, e);
                continue; // Skip this entry
            }
        };

        if !metadata.is_file() {
            continue;
        }
        
        if !canonical_entry_path.starts_with(&canonical_project_root) {
            error!(
                "Potential path traversal attempt detected: Canonical path {:?} is outside of canonical root {:?}",
                canonical_entry_path, canonical_project_root
            );
            continue;
        }


        let path_key = match canonical_entry_path.strip_prefix(&canonical_project_root) {
            Ok(relative_path) => {
                // Ensure it starts with a slash and use forward slashes for URL paths
                format!("/{}", relative_path.to_string_lossy().replace("\\", "/"))
            }
            Err(_) => {
                // This should ideally not happen if starts_with passed, but as a fallback
                error!(
                    "Failed to strip canonical root prefix from canonical path {:?} despite starts_with check",
                    canonical_entry_path
                );
                continue; // Skip this entry
            }
        };

        let full_path_for_cache = canonical_entry_path.to_string_lossy().into_owned();

        debug!("Caching file: {} -> {}", path_key, full_path_for_cache);

        // Insert into the paths map
        let mime_type = mime_guess::from_path(&canonical_entry_path)
            .first_or_text_plain()
            .to_string();

        paths.entry(path_key).or_insert(FileInfo {
            mime_type,
            full_path: full_path_for_cache,
        });
    }
}

pub async fn handle_request(
    request: Request<Incoming>,
) -> HostingResult<Response<BoxBody<Bytes, Infallible>>> {
    let project_slug = Slug::from_str(request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_suffix(HOSTING_PREFIX.as_str()))
        .unwrap_or_default()).map_err(|e|HostingError::Custom(e.to_string()))?;

    let base_path = request.uri().path();
    let path = if base_path.is_empty() || base_path == "/" {
        "/index.html"
    } else {
        base_path
    };
    let cache_entry = CACHE.get(&project_slug.to_project_slug_str()).and_then(|project_cache| {
        project_cache
            .paths
            .get(path)
            .map(|file_info| (file_info.clone(), project_cache.file_cache.clone()))
    });
    let (file_info, project_cache) = match cache_entry {
        Some(found) => found,
        None => return not_found_response(),
    };
    let (is_br, body) = match project_cache.get(path) {
        Some(cached_body) => (true, cached_body),
        None => {
            let buffer = match tokio::fs::read(&file_info.full_path).await {
                Ok(buf) => Bytes::from(buf),
                Err(e) => {
                    error!("Failed to read file {}: {}", file_info.full_path, e);
                    return internal_error_response();
                }
            };

            let full_path = file_info.full_path.clone();
            let path_clone = path.to_string();
            task::spawn(async move {
                match File::open(&full_path).await {
                    Ok(file) => {
                        let reader = BufReader::new(file);
                        let mut encoder = BrotliEncoder::new(reader);
                        let mut compressed_buffer = Vec::new();
                        if let Err(e) = encoder.read_to_end(&mut compressed_buffer).await {
                            error!("Failed to compress file {} with Brotli: {}", full_path, e);
                            return;
                        }
                        if let Err(e) = encoder.shutdown().await {
                            error!("Failed to shutdown Brotli encoder for {}: {}", full_path, e);
                            return;
                        }
                        project_cache.insert(path_clone, Bytes::from(compressed_buffer));
                    }
                    Err(e) => {
                        error!(
                            "Failed to open file {} for compression task: {}",
                            full_path, e
                        );
                    }
                }
            });
            (false, buffer) // Return uncompressed body for this request
        }
    };

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(SERVER, SERVER_HEADER.clone())
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(file_info.mime_type.as_str()).unwrap(),
        )
        .header(header::CACHE_CONTROL, CACHE_HEADER.clone());
    if is_br {
        response = response.header(header::CONTENT_ENCODING, CONTENT_ENCODING_BR.clone());
    }
    response
        .body(Full::new(body).boxed())
        .map_err(HostingError::from)
}

pub fn not_found_response() -> HostingResult<Response<BoxBody<Bytes, Infallible>>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Empty::new().boxed())
        .map_err(HostingError::from)
}

pub fn internal_error_response() -> HostingResult<Response<BoxBody<Bytes, Infallible>>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Empty::new().boxed())
        .map_err(HostingError::from)
}

pub fn create_socket(addr: SocketAddr) -> HostingResult<Socket> {
    let domain = match addr {
        SocketAddr::V4(_) => Domain::IPV4,
        SocketAddr::V6(_) => Domain::IPV6,
    };
    let addr = SockAddr::from(addr);
    let socket = Socket::new(domain, socket2::Type::STREAM, None)?;
    let backlog = 4096; // maximum number of pending connections
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_reuse_address(true)?;
    socket.set_nodelay(true)?;
    socket.set_nonblocking(true)?; // required for tokio
    socket.bind(&addr)?;
    socket.listen(backlog)?;

    Ok(socket)
}

pub async fn accept_hosting_loop(
    handle: runtime::Handle,
    listener: TcpListener,
) -> HostingResult<()> {
    let mut http = http1::Builder::new();
    http.pipeline_flush(true);

    let service = service_fn(handle_request);
    loop {
        let (stream, _) = listener.accept().await?;
        let http = http.clone();
        handle.spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(_e) = http.serve_connection(io, service).await {
                // ignore errors until https://github.com/hyperium/hyper/pull/3863/ is merged
                // This PR will allow us to filter out shutdown errors which are expected.
                // warn!("Connection error (this may be normal during shutdown): {e}");
            }
        });
    }
}
