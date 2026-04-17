//! Rate Limiting
//!
//! Rate limiting for API endpoints and operations.
//! Supports multi-level (minute/hour/day) limits with burst allowance.
//! When rate limit is exceeded, an audit event is logged if an AuditLogger is configured.
//!
//! # Performance
//!
//! Uses `DashMap` instead of `RwLock<HashMap>` for concurrent access.
//! Each key is independently locked (sharded), allowing concurrent checks
//! for different keys without blocking. This eliminates the global write lock
//! bottleneck that `RwLock<HashMap>` introduced.

use crate::error::Result;
use crate::security::{AuditEvent, AuditLogger};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
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

    /// Get the limit type name for audit logging
    pub fn limit_type(&self) -> &'static str {
        match self {
            RateLimitKey::Did(_) => "did",
            RateLimitKey::Ip(_) => "ip",
            RateLimitKey::ApiKey(_) => "api_key",
            RateLimitKey::Custom(_) => "custom",
        }
    }
}

/// Rate limit entry (internal tracking)
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

/// Rate limiter with audit logging
///
/// Uses `DashMap` for sharded concurrent access, eliminating the global
/// write-lock bottleneck of `RwLock<HashMap>`. Each key is locked
/// independently, allowing concurrent rate checks for different clients.
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: DashMap<String, RateLimitEntry>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: DashMap::new(),
            audit_logger: None,
        }
    }

    /// Create with default configuration
    pub fn default_limiter() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Set audit logger for security event tracking
    pub fn with_audit_logger(mut self, logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// Check if a request is allowed
    ///
    /// When rate limit is exceeded, an audit event (`RateLimitExceeded`) is logged
    /// and a `SecurityViolation` event may also be logged for repeated violations.
    ///
    /// This method is synchronous (non-async) since DashMap provides lock-free
    /// concurrent access without needing async runtime primitives.
    pub fn check(&self, key: &RateLimitKey) -> Result<RateLimitResult> {
        if !self.config.enabled {
            return Ok(RateLimitResult::Allowed);
        }

        let key_str = key.as_str().to_string();

        // DashMap entry API: get-or-create with atomic shard-level locking
        let mut entry = self
            .entries
            .entry(key_str)
            .or_insert_with(|| RateLimitEntry::new(self.config.burst_size));

        let now = Utc::now();

        // Update buckets
        self.update_buckets(&mut entry, now);

        // Check limits
        if entry.minute_count >= self.config.requests_per_minute {
            self.log_rate_exceeded(key, entry.minute_count, self.config.requests_per_minute);
            return Ok(RateLimitResult::Denied {
                reason: "Rate limit exceeded: too many requests per minute".to_string(),
                retry_after: 60 - (now - entry.minute_start).num_seconds() as u32,
            });
        }

        if entry.hour_count >= self.config.requests_per_hour {
            self.log_rate_exceeded(key, entry.hour_count, self.config.requests_per_hour);
            return Ok(RateLimitResult::Denied {
                reason: "Rate limit exceeded: too many requests per hour".to_string(),
                retry_after: 3600 - (now - entry.hour_start).num_seconds() as u32,
            });
        }

        if entry.day_count >= self.config.requests_per_day {
            self.log_rate_exceeded(key, entry.day_count, self.config.requests_per_day);
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

    /// Log a rate limit exceeded event
    fn log_rate_exceeded(&self, key: &RateLimitKey, current_rate: u32, limit: u32) {
        if let Some(logger) = &self.audit_logger {
            // Log RateLimitExceeded event
            if let Err(e) = logger.log(AuditEvent::RateLimitExceeded {
                identifier: key.as_str().to_string(),
                limit_type: key.limit_type().to_string(),
                current_rate,
                limit,
                timestamp: Utc::now(),
            }) {
                tracing::warn!("Failed to log rate limit audit event: {}", e);
            }

            // Log SecurityViolation for potential abuse detection
            if current_rate > limit * 2 {
                if let Err(e) = logger.log_security_violation(
                    "rate_limit_abuse",
                    &format!(
                        "Client {} exceeded rate limit by {}x (rate={}, limit={})",
                        key.as_str(),
                        current_rate / limit.max(1),
                        current_rate,
                        limit
                    ),
                    "high",
                ) {
                    tracing::warn!("Failed to log security violation audit event: {}", e);
                }
            }
        }
    }

    /// Record a request (for tracking purposes, without rate check)
    pub fn record_request(&self, key: &RateLimitKey) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key_str = key.as_str().to_string();

        let mut entry = self
            .entries
            .entry(key_str)
            .or_insert_with(|| RateLimitEntry::new(self.config.burst_size));

        let now = Utc::now();
        self.update_buckets(&mut entry, now);

        entry.minute_count += 1;
        entry.hour_count += 1;
        entry.day_count += 1;

        Ok(())
    }

    /// Reset rate limit for a key
    pub fn reset(&self, key: &RateLimitKey) -> Result<()> {
        self.entries.remove(key.as_str());
        Ok(())
    }

    /// Get current usage for a key
    pub fn get_usage(&self, key: &RateLimitKey) -> Result<RateLimitUsage> {
        if let Some(entry) = self.entries.get(key.as_str()) {
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

    /// Clean up expired entries using DashMap's `retain()` for efficient bulk removal
    pub fn cleanup(&self) -> Result<usize> {
        let now = Utc::now();
        let before = self.entries.len();

        self.entries.retain(|_, e| {
            // Keep entries that haven't expired (day bucket less than 24 hours old)
            (now - e.day_start).num_hours() <= 24
        });

        let removed = before - self.entries.len();
        Ok(removed)
    }

    /// Get rate limit configuration reference
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Request is denied
    Denied {
        /// Reason for denial
        reason: String,
        /// Seconds until client should retry
        retry_after: u32,
    },
}

impl RateLimitResult {
    /// Check if allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }

    /// Get retry_after seconds (0 if allowed)
    pub fn retry_after(&self) -> u32 {
        match self {
            RateLimitResult::Allowed => 0,
            RateLimitResult::Denied { retry_after, .. } => *retry_after,
        }
    }
}

/// Rate limit usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    use crate::security::audit::MemoryAuditSink;

    #[test]
    fn test_rate_limit_allowed() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        let result = limiter.check(&key).unwrap();
        assert!(result.is_allowed());
    }

    #[test]
    fn test_rate_limit_exceeded() {
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
        limiter.check(&key).unwrap();
        limiter.check(&key).unwrap();

        // Third should fail
        let result = limiter.check(&key).unwrap();
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_rate_limit_disabled() {
        let config = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        // Should always succeed when disabled
        for _ in 0..100 {
            let result = limiter.check(&key).unwrap();
            assert!(result.is_allowed());
        }
    }

    #[test]
    fn test_rate_limit_usage() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).unwrap();
        limiter.check(&key).unwrap();

        let usage = limiter.get_usage(&key).unwrap();
        assert_eq!(usage.minute_count, 2);
    }

    #[test]
    fn test_rate_limit_reset() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).unwrap();
        limiter.reset(&key).unwrap();

        let usage = limiter.get_usage(&key).unwrap();
        assert_eq!(usage.minute_count, 0);
    }

    #[test]
    fn test_rate_limit_key_types() {
        assert_eq!(RateLimitKey::Did("test".to_string()).limit_type(), "did");
        assert_eq!(RateLimitKey::Ip("127.0.0.1".to_string()).limit_type(), "ip");
        assert_eq!(
            RateLimitKey::ApiKey("key123".to_string()).limit_type(),
            "api_key"
        );
        assert_eq!(
            RateLimitKey::Custom("custom".to_string()).limit_type(),
            "custom"
        );
    }

    #[tokio::test]
    async fn test_audit_logger_on_rate_exceeded() {
        let sink = Arc::new(MemoryAuditSink::new(100));
        let logger = Arc::new(AuditLogger::new(sink.clone()));

        let config = RateLimitConfig {
            requests_per_minute: 2,
            requests_per_hour: 100,
            requests_per_day: 1000,
            burst_size: 0,
            enabled: true,
        };
        let limiter = RateLimiter::new(config).with_audit_logger(logger);

        let key = RateLimitKey::Ip("127.0.0.1".to_string());

        // First two succeed
        limiter.check(&key).unwrap();
        limiter.check(&key).unwrap();

        // Third exceeds limit — should log audit event
        let result = limiter.check(&key).unwrap();
        assert!(!result.is_allowed());

        let rate_events = sink.get_events_by_type("rate_limit_exceeded").await;
        assert_eq!(rate_events.len(), 1);
    }

    #[test]
    fn test_rate_limit_result_serialization() {
        let allowed = RateLimitResult::Allowed;
        let json = serde_json::to_string(&allowed).unwrap();
        assert_eq!(json, "\"Allowed\"");

        let denied = RateLimitResult::Denied {
            reason: "too many requests".to_string(),
            retry_after: 30,
        };
        let json = serde_json::to_string(&denied).unwrap();
        let decoded: RateLimitResult = serde_json::from_str(&json).unwrap();
        assert!(!decoded.is_allowed());
        assert_eq!(decoded.retry_after(), 30);
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_rate_limit_usage_unknown_key() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:unknown".to_string());

        let usage = limiter.get_usage(&key).unwrap();
        assert_eq!(usage.minute_count, 0);
        assert_eq!(usage.hour_count, 0);
        assert_eq!(usage.day_count, 0);
        assert_eq!(usage.burst_tokens, limiter.config().burst_size);
    }

    #[test]
    fn test_rate_limit_reset_unknown_key() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:unknown".to_string());

        // Resetting unknown key should succeed (just removes nothing)
        limiter.reset(&key).unwrap();
    }

    #[test]
    fn test_rate_limit_denied_retry_after() {
        let config = RateLimitConfig {
            requests_per_minute: 1,
            requests_per_hour: 100,
            requests_per_day: 1000,
            burst_size: 0,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).unwrap();
        let result = limiter.check(&key).unwrap();

        assert!(!result.is_allowed());
        assert!(result.retry_after() > 0);
        assert!(result.retry_after() <= 60);
    }

    #[test]
    fn test_rate_limit_different_keys_independent() {
        let config = RateLimitConfig {
            requests_per_minute: 2,
            requests_per_hour: 100,
            requests_per_day: 1000,
            burst_size: 0,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        let key1 = RateLimitKey::Did("did:nexa:user1".to_string());
        let key2 = RateLimitKey::Did("did:nexa:user2".to_string());

        // Exhaust key1
        limiter.check(&key1).unwrap();
        limiter.check(&key1).unwrap();
        let result1 = limiter.check(&key1).unwrap();
        assert!(!result1.is_allowed());

        // key2 should still be allowed (independent)
        let result2 = limiter.check(&key2).unwrap();
        assert!(result2.is_allowed());
    }

    #[test]
    fn test_rate_limit_result_allowed_retry_after() {
        let result = RateLimitResult::Allowed;
        assert_eq!(result.retry_after(), 0);
    }

    #[test]
    fn test_rate_limit_record_request() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.record_request(&key).unwrap();

        let usage = limiter.get_usage(&key).unwrap();
        assert_eq!(usage.minute_count, 1);
    }

    #[test]
    fn test_rate_limit_record_request_disabled() {
        let config = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        // Recording when disabled should succeed but not track
        limiter.record_request(&key).unwrap();
    }

    #[test]
    fn test_rate_limit_cleanup_no_expired() {
        let limiter = RateLimiter::default_limiter();
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).unwrap();

        // Immediately cleanup — nothing should be expired
        let count = limiter.cleanup().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_rate_limit_key_as_str() {
        let key = RateLimitKey::Did("did:nexa:test".to_string());
        assert_eq!(key.as_str(), "did:nexa:test");

        let key = RateLimitKey::Ip("192.168.1.1".to_string());
        assert_eq!(key.as_str(), "192.168.1.1");

        let key = RateLimitKey::ApiKey("abc123".to_string());
        assert_eq!(key.as_str(), "abc123");

        let key = RateLimitKey::Custom("my-key".to_string());
        assert_eq!(key.as_str(), "my-key");
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.requests_per_hour, 1000);
        assert_eq!(config.requests_per_day, 10000);
        assert_eq!(config.burst_size, 10);
        assert!(config.enabled);
    }

    #[test]
    fn test_rate_limit_hour_limit_exceeded() {
        let config = RateLimitConfig {
            requests_per_minute: 1000,
            requests_per_hour: 2,
            requests_per_day: 10000,
            burst_size: 0,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).unwrap();
        limiter.check(&key).unwrap();

        let result = limiter.check(&key).unwrap();
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_rate_limit_day_limit_exceeded() {
        let config = RateLimitConfig {
            requests_per_minute: 1000,
            requests_per_hour: 1000,
            requests_per_day: 2,
            burst_size: 0,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let key = RateLimitKey::Did("did:nexa:test".to_string());

        limiter.check(&key).unwrap();
        limiter.check(&key).unwrap();

        let result = limiter.check(&key).unwrap();
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_dashmap_concurrent_safety() {
        use std::sync::Arc;
        use std::thread;

        let limiter = Arc::new(RateLimiter::new(RateLimitConfig {
            requests_per_minute: 1000,
            requests_per_hour: 10000,
            requests_per_day: 100000,
            burst_size: 100,
            enabled: true,
        }));

        let mut handles = vec![];

        // 10 threads each making 100 rate limit checks with different keys
        for i in 0..10 {
            let limiter_clone = limiter.clone();
            handles.push(thread::spawn(move || {
                let key = RateLimitKey::Did(format!("did:nexa:thread{}", i));
                for _ in 0..100 {
                    assert!(limiter_clone.check(&key).unwrap().is_allowed());
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all threads' entries exist
        for i in 0..10 {
            let key = RateLimitKey::Did(format!("did:nexa:thread{}", i));
            let usage = limiter.get_usage(&key).unwrap();
            assert_eq!(usage.minute_count, 100);
        }
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Rate limit counter consistency: total count = minute + hour + day
        /// (all three always increment together)
        #[test]
        fn proptest_rate_limit_counter_consistency(
            num_requests in 1u32..5,
        ) {
            let config = RateLimitConfig {
                requests_per_minute: 100,
                requests_per_hour: 1000,
                requests_per_day: 10000,
                burst_size: 10,
                enabled: true,
            };
            let limiter = RateLimiter::new(config);
            let key = RateLimitKey::Did("did:nexa:proptest".to_string());

            for _ in 0..num_requests {
                limiter.check(&key).unwrap();
            }

            let usage = limiter.get_usage(&key).unwrap();
            assert_eq!(usage.minute_count, num_requests);
            assert_eq!(usage.hour_count, num_requests);
            assert_eq!(usage.day_count, num_requests);
        }
    }
}
