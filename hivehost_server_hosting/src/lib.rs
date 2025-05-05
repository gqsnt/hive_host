use async_compression::tokio::bufread::BrotliEncoder;
use common::PROD_ROOT_PATH_PREFIX;
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
use std::sync::{Arc, LazyLock};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::{runtime, task};
use tracing::{debug, error, info};
use walkdir::WalkDir;

pub mod handler;

pub static HOSTING_PREFIX: LazyLock<String> = LazyLock::new(|| {
    format!(
        ".{}",
        dotenvy::var("HOSTING_URL").expect("HOSTING_URL must be set")
    )
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
        // Be cautions out about zero weights!
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
    info!("Serving/Caching Project: {}", project_slug);
    let path = get_project_prod_path(&project_slug);

    CACHE.remove(&project_slug);
    let full_root_path = format!("{PROD_ROOT_PATH_PREFIX}/");
    let entry = CACHE.entry(project_slug).or_default();
    WalkDir::new(path)
        .into_iter()
        .filter_map(|dir_entry| dir_entry.ok())
        .filter(|dir_entry| dir_entry.metadata().unwrap().is_file())
        .for_each(|dir_entry| {
            let mime_type = mime_guess::from_path(dir_entry.path())
                .first_or_text_plain()
                .to_string();
            let full_path = dir_entry.path().to_string_lossy().to_string();
            let path = full_path.strip_prefix(&full_root_path).unwrap();

            let path_split = path.split('/').collect::<Vec<_>>();
            let path = format!("/{}", path_split[1..].join("/"));
            debug!("Caching file: {} -> {}", path, full_path);
            entry.paths.entry(path).or_insert(FileInfo {
                mime_type,
                full_path,
            });
        });
}

pub async fn handle_request(
    request: Request<Incoming>,
) -> HostingResult<Response<BoxBody<Bytes, Infallible>>> {
    let project_slug = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_suffix(HOSTING_PREFIX.as_str()))
        .unwrap_or_default();
    if project_slug.is_empty() {
        return not_found_response();
    }
    let base_path = request.uri().path();
    let path = if base_path.is_empty() || base_path == "/" {
        "/index.html"
    } else {
        base_path
    };
    let cache_entry = CACHE.get(project_slug).and_then(|project_cache| {
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
                    // Return Internal Server Error
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
                        // Use AsyncReadExt::read_to_end
                        if let Err(e) = encoder.read_to_end(&mut compressed_buffer).await {
                            error!("Failed to compress file {} with Brotli: {}", full_path, e);
                            return; // Don't insert partial data
                        }
                        // Shutdown is important for BrotliEncoder
                        if let Err(e) = encoder.shutdown().await {
                            error!("Failed to shutdown Brotli encoder for {}: {}", full_path, e);
                            return; // Don't insert potentially corrupt data
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
