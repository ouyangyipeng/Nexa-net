//! Rate Limiting
//!
//! Rate limiting for API endpoints and operations.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum requests per hour
    pub requests_per_hour: u32,
    /// Maximum requests per day
    pub requests_per_day: u32,
    /// Burst size (allows temporary spikes)
    pub burst_size: u32,
    /// Enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_size: 10,
            enabled: true,
        }
    }
}

/// Rate limit key type
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RateLimitKey {
    /// By DID
    Did(String),
    /// By IP address
    Ip(String),
    /// By API key
    ApiKey(String),
    /// Custom identifier
    Custom(String),
}

impl RateLimitKey {
    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            RateLimitKey::Did(s) => s,
            RateLimitKey::Ip(s) => s,
            RateLimitKey::ApiKey(s) => s,
            RateLimitKey::Custom(s) => s,
        }
    }
}

/// Rate limit entry
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Minute bucket
    minute_count: u32,
    /// Minute bucket start
    minute_start: DateTime<Utc>,
    /// Hour bucket
    hour_count: u32,
    /// Hour bucket start
    hour_start: DateTime<Utc>,
    /// Day bucket
    day_count: u32,
    /// Day bucket start
    day_start: DateTime<Utc>,
    /// Burst tokens available
    burst_tokens: u32,
    /// Last burst refill
    last_refill: DateTime<Utc>,
}

impl RateLimitEntry {
    fn new(burst_size: u32) -> Self {
        let now = Utc::now();
        Self {
            minute_count: 0,
            minute_start: now,
            hour_count: 0,
            hour_start: now,
            day_count: 0,
            day_start: now,
            burst_tokens: burst_size,
            last_refill: now,
        }
    }
}

/// Rate limiter
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn default_limiter() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check if a request is allowed
    pub async fn check(&self, key: &RateLimitKey) -> Result<RateLimitResult> {
        if !self.config.enabled {
            return Ok(RateLimitResult::Allowed);
        }

        let mut entries = self.entries.write().await;
        let key_str = key.as_str().to_string();

        let entry = entries
            .entry(key_str.clone())
            .or_insert_with(|| RateLimitEntry::new(self.config.burst_size));

        let now = Utc::now();

        // Update buckets
        self.update_buckets(entry, now);

        // Check limits
        if entry.minute_count >= self.config.requests_per_minute {
            return Ok(RateLimitResult::Denied {
                reason: "Rate limit exceeded: too many requests per minute".to_string(),
                retry_after: 60 - (now - entry.minute_start).num_seconds() as u32,
            });
        }

        if entry.hour_count >= self.config.requests_per_hour {
            return Ok(RateLimitResult::Denied {
                reason: "Rate limit exceeded: too many requests per hour".to_string(),
                retry_after: 3600 - (now - entry.hour_start).num_seconds() as u32,
            });
        }

        if entry.day_count >= self.config.requests_per_day {
            return Ok(RateLimitResult::Denied {
                reason: "Rate limit exceeded: too many requests per day".to_string(),
                retry_after: 86400 - (now - entry.day_start).num_seconds() as u32,
            });
        }

        // Use burst token if available, otherwise increment counters
        if entry.burst_tokens > 0 {
            entry.burst_tokens -= 1;
        }

        entry.minute_count += 1;
        entry.hour_count += 1;
        entry.day_count += 1;

        Ok(RateLimitResult::Allowed)
    }

    /// Record a request (for tracking purposes)
    pub async fn record_request(&self, key: &RateLimitKey) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut entries = self.entries.write().await;
        let key_str = key.as_str().to_string();

        let entry = entries
            .entry(key_str)
            .or_insert_with(|| RateLimitEntry::new(self.config.burst_size));

        let now = Utc::now();
        self.update_buckets(entry, now);

        entry.minute_count += 1;
        entry.hour_count += 1;
        entry.day_count += 1;

        Ok(())
    }

    /// Reset rate limit for a key
    pub async fn reset(&self, key: &RateLimitKey) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.remove(key.as_str());
        Ok(())
    }

    /// Get current usage for a key
    pub async fn get_usage(&self, key: &RateLimitKey) -> Result<RateLimitUsage> {
        let entries = self.entries.read().await;

        if let Some(entry) = entries.get(key.as_str()) {
            Ok(RateLimitUsage {
                minute_count: entry.minute_count,
                minute_limit: self.config.requests_per_minute,
                hour_count: entry.hour_count,
                hour_limit: self.config.requests_per_hour,
                day_count: entry.day_count,
                day_limit: self.config.requests_per_day,
                burst_tokens: entry.burst_tokens,
            })
        } else {
            Ok(RateLimitUsage {
                minute_count: 0,
                minute_limit: self.config.requests_per_minute,
                hour_count: 0,
                hour_limit: self.config.requests_per_hour,
                day_count: 0,
                day_limit: self.config.requests_per_day,
                burst_tokens: self.config.burst_size,
            })
        }
    }

    /// Clean up expired entries
    pub async fn cleanup(&self) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let now = Utc::now();

        let expired: Vec<_> = entries
            .iter()
            .filter(|(_, e)| (now - e.day_start).num_hours() > 24)
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired.len();
        for key in expired {
            entries.remove(&key);
        }

        Ok(count)
    }

    /// Update time buckets
    fn update_buckets(&self, entry: &mut RateLimitEntry, now: DateTime<Utc>) {
        // Update minute bucket
        if (now - entry.minute_start).num_seconds() >= 60 {
            entry.minute_count = 0;
            entry.minute_start = now;
        }

        // Update hour bucket
        if (now - entry.hour_start).num_seconds() >= 3600 {
            entry.hour_count = 0;
            entry.hour_start = now;
        }

        // Update day bucket
        if (now - entry.day_start).num_seconds() >= 86400 {
            entry.day_count = 0;
            entry.day_start = now;
        }

        // Refill burst tokens (1 token per second)
        let seconds_since_refill = (now - entry.last_refill).num_seconds() as u32;
        if seconds_since_refill > 0 {
            entry.burst_tokens =
                (entry.burst_tokens + seconds_since_refill).min(self.config.burst_size);
            entry.last_refill = now;
        }
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Request is denied
    Denied { reason: String, retry_after: u32 },
}

impl RateLimitResult {
    /// Check if allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }
}

/// Rate limit usage information
#[derive(Debug, Clone)]
pub struct RateLimitUsage {
    pub minute_count: u32,
    pub minute_limit: u32,
    pub hour_count: u32,
    pub hour_limit: u32,
    pub day_count: u32,
    pub day_limit: u32,
    pub burst_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_allowed() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        let result = limiter.check(&key).await.unwrap();
        assert!(result.is_allowed());
    }

    #[tokio::test]
    async fn test_rate_limit_exceeded() {
        let config = RateLimitConfig {
            requests_per_minute: 2,
            requests_per_hour: 100,
            requests_per_day: 1000,
            burst_size: 0,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        // First two should succeed
        limiter.check(&key).await.unwrap();
        limiter.check(&key).await.unwrap();

        // Third should fail
        let result = limiter.check(&key).await.unwrap();
        assert!(!result.is_allowed());
    }

    #[tokio::test]
    async fn test_rate_limit_disabled() {
        let config = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        // Should always succeed when disabled
        for _ in 0..100 {
            let result = limiter.check(&key).await.unwrap();
            assert!(result.is_allowed());
        }
    }

    #[tokio::test]
    async fn test_rate_limit_usage() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).await.unwrap();
        limiter.check(&key).await.unwrap();

        let usage = limiter.get_usage(&key).await.unwrap();
        assert_eq!(usage.minute_count, 2);
    }

    #[tokio::test]
    async fn test_rate_limit_reset() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).await.unwrap();
        limiter.reset(&key).await.unwrap();

        let usage = limiter.get_usage(&key).await.unwrap();
        assert_eq!(usage.minute_count, 0);
    }
}
