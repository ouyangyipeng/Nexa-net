//! Rate Limit Middleware for Axum
//!
//! Provides an Axum middleware layer that enforces rate limiting on API requests.
//! Extracts rate limit keys from request headers (X-DID, X-API-Key) or falls
//! back to the client IP address.
//!
//! Note: Since RateLimiter now uses DashMap (synchronous, lock-free),
//! the check is performed synchronously without async overhead.

use crate::security::{RateLimitConfig, RateLimitKey, RateLimitResult, RateLimiter};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Rate limit middleware state shared via Axum State
///
/// Wraps a `RateLimiter` and provides configuration for key extraction.
#[derive(Clone)]
pub struct RateLimitMiddleware {
    /// The rate limiter instance
    limiter: Arc<RateLimiter>,
}

impl RateLimitMiddleware {
    /// Create middleware from a RateLimiter
    pub fn new(limiter: RateLimiter) -> Self {
        Self {
            limiter: Arc::new(limiter),
        }
    }

    /// Create middleware from config
    pub fn from_config(config: RateLimitConfig) -> Self {
        Self::new(RateLimiter::new(config))
    }

    /// Get the rate limiter reference
    pub fn limiter(&self) -> &RateLimiter {
        &self.limiter
    }
}

/// Extract rate limit key from request headers
///
/// Priority:
/// 1. `X-DID` header (if present)
/// 2. `X-API-Key` header (if present)
/// 3. Client IP from `X-Real-IP` or `X-Forwarded-For` headers
/// 4. Falls back to "unknown" if no identifier can be extracted
fn extract_rate_limit_key(headers: &HeaderMap) -> RateLimitKey {
    // Check X-DID header first
    if let Some(did) = headers.get("X-DID") {
        if let Ok(did_str) = did.to_str() {
            return RateLimitKey::Did(did_str.to_string());
        }
    }

    // Check X-API-Key header
    if let Some(api_key) = headers.get("X-API-Key") {
        if let Ok(key_str) = api_key.to_str() {
            return RateLimitKey::ApiKey(key_str.to_string());
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() {
            return RateLimitKey::Ip(ip_str.to_string());
        }
    }

    // Check X-Forwarded-For header (first IP in the list)
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return RateLimitKey::Ip(first_ip.trim().to_string());
            }
        }
    }

    RateLimitKey::Ip("unknown".to_string())
}

/// Axum middleware handler for rate limiting
///
/// The rate limit check is now synchronous (DashMap provides lock-free access),
/// eliminating async overhead for the most common middleware operation.
///
/// Returns 429 Too Many Requests with `Retry-After` header if rate exceeded.
pub async fn rate_limit_middleware(
    State(rate_limit): State<Arc<RateLimitMiddleware>>,
    request: Request,
    next: Next,
) -> Response {
    // Extract headers from the request before forwarding
    let headers = request.headers().clone();

    // Extract rate limit key from headers
    let key = extract_rate_limit_key(&headers);

    // Check rate limit — synchronous since DashMap is lock-free
    let result = match rate_limit.limiter.check(&key) {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Rate limit check failed: {}", e);
            // Allow request on internal error (fail-open)
            return next.run(request).await;
        }
    };

    match result {
        RateLimitResult::Allowed => next.run(request).await,
        RateLimitResult::Denied {
            reason,
            retry_after,
        } => {
            tracing::warn!(
                reason = %reason,
                retry_after = retry_after,
                key = %key.as_str(),
                "Rate limit exceeded"
            );

            // Return 429 Too Many Requests with Retry-After header
            let body = Body::from(format!(
                "{{\"error\":\"rate_limit_exceeded\",\"reason\":\"{}\",\"retry_after\":{}}}",
                reason, retry_after
            ));

            (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_after.to_string())],
                [("Content-Type", "application/json")],
                body,
            )
                .into_response()
        }
    }
}
