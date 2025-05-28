use async_trait::async_trait;
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};
use poem::web::RealIp;
use poem::{http::StatusCode, middleware::Middleware, Endpoint, Request, Response};
use poem::{FromRequest, IntoResponse};
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{info, warn};
// Type aliases for clarity.
type IpRateLimiter = RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock>;
type EndpointIpRateLimiter =
    RateLimiter<(String, IpAddr), DashMapStateStore<(String, IpAddr)>, DefaultClock>;

#[derive(Clone)]
pub struct GovRateLimitMiddleware {
    ip_limiter: Arc<IpRateLimiter>,
    endpoint_ip_limiter: Arc<EndpointIpRateLimiter>,
}

impl GovRateLimitMiddleware {
    /// Create a new middleware.
    /// `ip_quota` is the site-wide quota per IP.
    /// `endpoint_ip_quota` is the quota per (endpoint, IP) pair.
    #[must_use]
    pub fn new(ip_quota: Quota, endpoint_ip_quota: Quota) -> Self {
        let ip_limiter = RateLimiter::keyed(ip_quota);
        let endpoint_ip_limiter = RateLimiter::keyed(endpoint_ip_quota);
        Self {
            ip_limiter: Arc::new(ip_limiter),
            endpoint_ip_limiter: Arc::new(endpoint_ip_limiter),
        }
    }
}

#[async_trait]
impl<E: Endpoint> Middleware<E> for GovRateLimitMiddleware {
    type Output = GovRateLimitMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        GovRateLimitMiddlewareImpl {
            ep,
            ip_limiter: self.ip_limiter.clone(),
            endpoint_ip_limiter: self.endpoint_ip_limiter.clone(),
        }
    }
}

pub struct GovRateLimitMiddlewareImpl<E> {
    ep: E,
    ip_limiter: Arc<IpRateLimiter>,
    endpoint_ip_limiter: Arc<EndpointIpRateLimiter>,
}

impl<E: Endpoint> Endpoint for GovRateLimitMiddlewareImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        // Extract IP (fallback to 127.0.0.1)
        let ip = RealIp::from_request_without_body(&req).await?;
        let ip = ip.0.unwrap_or_else(|| {
            warn!("Failed to parse IP address for rate limiting");
            "127.0.0.1".parse().unwrap()
        });

        let endpoint = req.uri().to_string();
        info!("Rate limiting request to endpoint: {}", endpoint);

        // Check the site-wide rate limit for the IP.
        if self.ip_limiter.check_key(&ip).is_err() {
            warn!("Site-wide rate limit exceeded for IP: {}", ip);
            return Ok(Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body("Site-wide rate limit exceeded for IP"));
        }

        // Check the endpoint-specific rate limit for (endpoint, IP).
        let key = (endpoint.clone(), ip);
        if self.endpoint_ip_limiter.check_key(&key).is_err() {
            warn!("Endpoint-specific rate limit exceeded for IP: {}", ip);
            return Ok(Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body("Endpoint-specific rate limit exceeded for IP"));
        }

        // Process the request normally.
        let x = self.ep.call(req).await?;

        Ok(x.into_response())
    }
}
