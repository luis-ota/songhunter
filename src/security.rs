use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const ALLOWED_ORIGIN: &str = "https://songhunter.wired.rs";

/// In-memory sliding-window rate limiter per client IP (hashed).
pub struct RateLimiter {
    inner: Mutex<HashMap<u64, Vec<Instant>>>,
    max_requests: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check(&self, ip: u64) -> bool {
        let mut map = self.inner.lock().unwrap();
        let now = Instant::now();
        let cutoff = now - self.window;

        let timestamps = map.entry(ip).or_default();
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.max_requests as usize {
            return false;
        }

        timestamps.push(now);
        true
    }
}

fn client_ip_hash(req: &Request) -> Option<u64> {
    let ip_str = req
        .headers()
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next().map(|s| s.trim()))
        })?;

    let addr: std::net::IpAddr = ip_str.parse().ok()?;
    match addr {
        std::net::IpAddr::V4(v4) => Some(u64::from(u32::from(v4))),
        std::net::IpAddr::V6(v6) => {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            v6.segments().hash(&mut h);
            Some(h.finish())
        }
    }
}

/// Middleware: validates Origin/Referer for POST /api/identify and rate-limits.
pub async fn api_guard(
    State(limiter): State<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let method = req.method().clone();

    if method == Method::POST && path == "/api/identify" {
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());

        let referer = req
            .headers()
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok());

        let valid = origin.is_some_and(|o| o == ALLOWED_ORIGIN)
            || referer.is_some_and(|r| {
                r == ALLOWED_ORIGIN || r.starts_with(&format!("{}/", ALLOWED_ORIGIN))
            });

        if !valid {
            tracing::warn!(
                "blocked unauthorized identify request: origin={:?} referer={:?}",
                origin,
                referer
            );
            return Err(StatusCode::FORBIDDEN);
        }

        if let Some(ip) = client_ip_hash(&req) {
            if !limiter.check(ip) {
                tracing::warn!("rate limited client IP hash={}", ip);
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
    }

    Ok(next.run(req).await)
}
