use std::fs::File;
use std::io;
use std::io::Read;
use std::sync::Arc;

// Use Arc for cheap cloning
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use dashmap::DashMap;
use http::header::HOST;
use may_minihttp::{HttpService, HttpServiceFactory, Request, Response};
use quick_cache::sync::Cache;
use walkdir::WalkDir;

const PROJECT_ROOT_PATH: &str = "/projects";
const PROJECT_ROOT_PATH_PREFIX: &str = "/projects/";
const PROJECT_ROOT_PREFIX: &str = "projects.";


struct HostService{
    path_cache: Arc<DashMap<(String, String), String>>,
    file_cache :Arc<Cache<(String,String), Vec<u8>>>,
    suffix_hosting_url:String,
}

impl HttpService for HostService {
    fn call(&mut self, req: Request, rsp: &mut Response) -> std::io::Result<()> {
        let path = req.path().to_string();
        let host = req.headers().iter()
            .find(|h| h.name == HOST)
            .map(|h| String::from_utf8_lossy(h.value).to_string())
            .unwrap_or_default();
        let project = host.strip_suffix(self.suffix_hosting_url.as_str()).unwrap_or_default().to_string();
        let ref_ = (project, path);
        match  self.path_cache.get(&ref_){
            None => {
                rsp.status_code(404, "Not Found");
            }
            Some(path_ref) => {
                let path = path_ref.value().to_string();
                let cache_key = ref_.clone(); // Clone key for cache lookup/insert
                drop(path_ref);

                let body = if let Some(cached_file_arc_bytes) = self.file_cache.get(&cache_key) {
                    // Cache hit: Clone the Arc<Bytes> cheaply
                    cached_file_arc_bytes
                } else {
                    // Cache miss: Read asynchronously
                    let mut file = File::open(path)?; // Async open
                    let mut encoder = zstd::stream::Encoder::new(Vec::new(), 4).unwrap();
                    let _ = io::copy(&mut file, &mut encoder).unwrap();
                    let buffer = encoder.finish().unwrap();
                    self.file_cache.insert(cache_key,buffer.clone());
                
                    // Return the Arc<Bytes> for the response
                    buffer
                };
                //self.file_cache.insert(cache_key,body.clone());

                // Send the body. body_ref takes AsRef<[u8]>, which Arc<Bytes> should implement via Deref
                // If body_ref doesn't exist or doesn't work, you might need body_vec(body_bytes.to_vec())
                // or access the underlying slice: rsp.body(&body_bytes);
                rsp.body_vec(body);
                rsp.header("Content-Type: text/html;charset=utf8");
                rsp.header("Content-Encoding: zstd");
            }
        }
        Ok(())
    }
}


struct HttpServer {
    path_cache: Arc<DashMap<(String, String), String>>,
    file_cache :Arc<Cache<(String,String), Vec<u8>>>,
    suffix_hosting_url:String,
}


impl HttpServiceFactory for HttpServer {
    type Service = HostService;

    fn new_service(&self, id: usize) -> Self::Service {
        let path_cache=  self.path_cache.clone();
        let file_cache=  self.file_cache.clone();
        let suffix_hosting_url = self.suffix_hosting_url.clone();
        HostService { path_cache, suffix_hosting_url, file_cache }
    }
}

fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");
    let hosting_url = dotenvy::var("HOSTING_URL").expect("HOSTING_URL must be set");
    let hosting_addr = dotenvy::var("HOSTING_ADDR").expect("HOSTING_URL must be set");

    may::config().set_pool_capacity(1000).set_stack_size(0x1000);
    println!("Starting http server: {}", hosting_url);
    let path_cache = DashMap::new();
    let project_prefix = format!("{}/", PROJECT_ROOT_PREFIX);
    WalkDir::new(PROJECT_ROOT_PATH)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().unwrap().is_file())
        .for_each(|entry| {
            let full_path = entry.path().to_string_lossy().to_string();
            let path = full_path.strip_prefix(PROJECT_ROOT_PATH_PREFIX).unwrap();

            let path_split = path.split('/').collect::<Vec<_>>();

            let project_path = path_split[0].to_string();

            let path = format!("/{}", path_split[1..].join("/"));
            path_cache.insert((project_path, path), entry.path().to_string_lossy().to_string());
        });
    let file_cache = Cache::new(5000);
    let server = HttpServer {
        path_cache: Arc::new(path_cache),
        file_cache: Arc::new(file_cache),
        suffix_hosting_url:format!(".{}", hosting_url),
    };
    server.start(hosting_addr).unwrap().join().unwrap();
}

