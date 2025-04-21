#[cfg(feature = "ssr")]
pub mod ssr {
    use axum::body::Body;
    use axum::extract::{ConnectInfo, Request, State};
    use axum::middleware::Next;
    use axum::response::Response;
    use http::StatusCode;
    use moka::future::Cache;
    use std::collections::VecDeque;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[derive(Clone, Debug)]
    pub struct SlidingWindowEntry {
        pub timestamps: VecDeque<Instant>,
    }

    impl Default for SlidingWindowEntry {
        fn default() -> Self {
            Self {
                timestamps: VecDeque::with_capacity(100), // pré-allocation possible
            }
        }
    }

    #[derive(Clone)]
    pub struct SlidingWindowRateLimiter {
        pub cache: Cache<String, SlidingWindowEntry>,
        pub limit_per_minute: u32,
        pub window: Duration,
    }

    impl SlidingWindowRateLimiter {
        /// Crée un rate-limiter capable de conserver 1000 IP
        /// et un TTL global pour nettoyer les IP inactives.
        pub fn new(limit_per_minute: u32) -> Self {
            let cache = Cache::builder()
                // on peut mettre un time_to_live plus grand que la fenêtre, pour enlever la data
                // si inactif 5 minutes, par ex.
                .time_to_live(Duration::from_secs(120))
                .max_capacity(1000)
                .build();
            SlidingWindowRateLimiter {
                cache,
                limit_per_minute,
                window: Duration::from_secs(60),
            }
        }

        /// Retourne `Ok(())` si on est sous la limite,
        /// ou `Err(())` si on doit bloquer la requête.
        pub async fn check(&self, ip: &str) -> bool {
            // Récupère ou initialise la file des timestamps pour cette IP
            let now = Instant::now();
            let mut entry = self.cache.get(ip).await.unwrap_or_default();

            // Purge les timestamps plus vieux que `self.window` (60s)
            while let Some(&front_ts) = entry.timestamps.front() {
                if now.duration_since(front_ts) > self.window {
                    entry.timestamps.pop_front();
                } else {
                    break;
                }
            }

            // Vérifie si on atteint la limite
            if entry.timestamps.len() as u32 >= self.limit_per_minute {
                // on refuse
                return false;
            }

            // Sinon, on ajoute le timestamp courant
            entry.timestamps.push_back(now);
            // on ré-insère l'objet dans le cache
            self.cache.insert(ip.to_string(), entry).await;

            true
        }
    }

    #[derive(Clone)]
    pub struct RateLimiter {
        // Pour /login, /signup : 5 req/min
        pub auth_cache: SlidingWindowRateLimiter,
        // Pour le reste : 100 req/min
        pub default_cache: SlidingWindowRateLimiter,
    }

    impl Default for RateLimiter {
        fn default() -> Self {
            let auth_cache = SlidingWindowRateLimiter::new(5);
            let default_cache = SlidingWindowRateLimiter::new(100);
            Self {
                auth_cache,
                default_cache,
            }
        }
    }

    impl RateLimiter {
        pub async fn check_limit(&self, ip: &str, is_auth_endpoint: bool) -> bool {
            if is_auth_endpoint {
                self.auth_cache.check(ip).await
            } else {
                self.default_cache.check(ip).await
            }
        }
    }

    pub async fn rate_limit_middleware(
        State(rate_limiter): State<Arc<RateLimiter>>,
        connect_info: ConnectInfo<SocketAddr>,
        req: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // Récupérer l'IP
        let ip = connect_info.ip();

        let is_auth_endpoint = req.uri().path().starts_with("/api/login")
            || req.uri().path().starts_with("/api/signup");
        let ip_str = ip.to_string();
        if !rate_limiter.check_limit(&ip_str, is_auth_endpoint).await {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        Ok(next.run(req).await)
    }
}
