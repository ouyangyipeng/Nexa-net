//! Security Module
//!
//! Provides security enhancements for Nexa-net:
//! - **Secure key storage** with AES-256-GCM encryption (now using DashMap)
//! - **Key rotation management** with configurable policies
//! - **Audit logging** for all security events
//! - **Rate limiting** with multi-level (min/hour/day) + burst tokens (now using DashMap)
//! - **Axum middleware** for API rate limiting
//!
//! # Architecture
//!
//! The `SecurityManager` coordinates all security subsystems:
//! - It holds references to `SecureKeyStorage`, `KeyRotator`, `RateLimiter`,
//!   and `AuditLogger`
//! - All subsystems share the same `AuditLogger` so security events are
//!   centrally collected
//! - The `SecurityManager` is designed to be injected into the Axum router
//!   as shared state
//!
//! # Performance
//!
//! Both `RateLimiter` and `SecureKeyStorage` use `DashMap` instead of
//! `RwLock<HashMap>`, providing sharded concurrent access without global
//! write-lock bottlenecks. All methods on these subsystems are now
//! synchronous (non-async), eliminating async runtime scheduling overhead.

pub mod audit;
pub mod key_rotation;
pub mod middleware;
pub mod rate_limit;
pub mod secure_storage;

// Re-exports
pub use audit::{AuditEvent, AuditLogger, AuditSink, AuthMethod, ChannelState, MemoryAuditSink};
pub use key_rotation::{
    KeyMetadata as RotationKeyMetadata, KeyRotationPolicy, KeyRotator, RotationStats,
};
pub use middleware::{rate_limit_middleware, RateLimitMiddleware};
pub use rate_limit::{RateLimitConfig, RateLimitKey, RateLimitResult, RateLimitUsage, RateLimiter};
pub use secure_storage::{
    KeyMetadata as StorageKeyMetadata, SecureKeyStorage, StorageStats as SecureStorageStats,
};

use std::sync::Arc;

/// Security configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityConfig {
    /// Enable audit logging
    pub audit_enabled: bool,
    /// Key rotation policy
    pub key_rotation: KeyRotationPolicy,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// Encryption key for secure storage (should be loaded from secure source)
    /// Stored as base64 string for serialization; decoded to [u8; 32] on use
    pub storage_encryption_key_b64: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            audit_enabled: true,
            key_rotation: KeyRotationPolicy::default(),
            rate_limit: RateLimitConfig::default(),
            storage_encryption_key_b64: None,
        }
    }
}

impl SecurityConfig {
    /// Decode the base64 encryption key to raw bytes
    pub fn decode_encryption_key(&self) -> SecurityResult<Option<[u8; 32]>> {
        use base64::Engine;
        match &self.storage_encryption_key_b64 {
            Some(b64) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| {
                        SecurityError::EncryptionError(format!("Invalid base64 key: {}", e))
                    })?;
                if bytes.len() != 32 {
                    return Err(SecurityError::EncryptionError(format!(
                        "Encryption key must be 32 bytes, got {}",
                        bytes.len()
                    )));
                }
                let key: [u8; 32] = bytes.try_into().map_err(|_| {
                    SecurityError::EncryptionError("Failed to convert key to 32 bytes".to_string())
                })?;
                Ok(Some(key))
            }
            None => Ok(None),
        }
    }
}

/// Security error types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Key rotation failed: {0}")]
    RotationFailed(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Audit log error: {0}")]
    AuditError(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

impl From<SecurityError> for crate::error::Error {
    fn from(e: SecurityError) -> Self {
        crate::error::Error::Internal(e.to_string())
    }
}

/// Security result type
pub type SecurityResult<T> = std::result::Result<T, SecurityError>;

/// Security Manager — coordinates all security subsystems
///
/// The `SecurityManager` is the central point for security operations.
/// It holds references to all subsystems (key storage, rotation, rate limiting,
/// audit logging) and ensures they share the same audit logger.
///
/// Since `RateLimiter` and `SecureKeyStorage` now use DashMap (synchronous,
/// lock-free), their methods no longer require async runtime.
pub struct SecurityManager {
    /// Secure key storage
    secure_storage: SecureKeyStorage,
    /// Key rotator
    key_rotator: KeyRotator,
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Audit logger
    audit_logger: Arc<AuditLogger>,
    /// Security configuration
    config: SecurityConfig,
}

impl SecurityManager {
    /// Create a new SecurityManager from configuration
    ///
    /// Initializes all subsystems with the same AuditLogger so
    /// security events flow to a single sink.
    pub fn new(config: SecurityConfig) -> SecurityResult<Self> {
        let audit_logger = Arc::new(AuditLogger::with_logging());

        // Decode encryption key if provided
        let encryption_key = config.decode_encryption_key()?;

        // Initialize secure storage with audit logger
        let secure_storage =
            SecureKeyStorage::new(encryption_key).with_audit_logger(audit_logger.clone());

        // Initialize key rotator with audit logger
        let key_rotator =
            KeyRotator::new(config.key_rotation.clone()).with_audit_logger(audit_logger.clone());

        // Initialize rate limiter with audit logger
        let rate_limiter =
            RateLimiter::new(config.rate_limit.clone()).with_audit_logger(audit_logger.clone());

        Ok(Self {
            secure_storage,
            key_rotator,
            rate_limiter,
            audit_logger,
            config,
        })
    }

    /// Create with a custom audit sink (e.g., MemoryAuditSink for testing)
    pub fn with_audit_sink(
        sink: Arc<dyn AuditSink>,
        config: SecurityConfig,
    ) -> SecurityResult<Self> {
        let audit_logger = Arc::new(AuditLogger::new(sink));
        let encryption_key = config.decode_encryption_key()?;

        let secure_storage =
            SecureKeyStorage::new(encryption_key).with_audit_logger(audit_logger.clone());

        let key_rotator =
            KeyRotator::new(config.key_rotation.clone()).with_audit_logger(audit_logger.clone());

        let rate_limiter =
            RateLimiter::new(config.rate_limit.clone()).with_audit_logger(audit_logger.clone());

        Ok(Self {
            secure_storage,
            key_rotator,
            rate_limiter,
            audit_logger,
            config,
        })
    }

    /// Get the secure key storage reference
    pub fn secure_storage(&self) -> &SecureKeyStorage {
        &self.secure_storage
    }

    /// Get the key rotator reference
    pub fn key_rotator(&self) -> &KeyRotator {
        &self.key_rotator
    }

    /// Get the rate limiter reference
    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }

    /// Get the audit logger reference
    pub fn audit_logger(&self) -> &Arc<AuditLogger> {
        &self.audit_logger
    }

    /// Get the security configuration reference
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Log a custom security violation event
    pub fn log_security_violation(
        &self,
        violation_type: &str,
        details: &str,
        severity: &str,
    ) -> SecurityResult<()> {
        if let Err(e) = self
            .audit_logger
            .log_security_violation(violation_type, details, severity)
        {
            tracing::warn!("Failed to log security violation audit event: {}", e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::audit::MemoryAuditSink;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.audit_enabled);
        assert!(config.storage_encryption_key_b64.is_none());
    }

    #[test]
    fn test_security_config_decode_key_none() {
        let config = SecurityConfig::default();
        assert!(config.decode_encryption_key().unwrap().is_none());
    }

    #[test]
    fn test_security_config_decode_key_valid() {
        use base64::Engine;
        let key = [42u8; 32];
        let b64 = base64::engine::general_purpose::STANDARD.encode(key);

        let config = SecurityConfig {
            storage_encryption_key_b64: Some(b64),
            ..Default::default()
        };

        let decoded = config.decode_encryption_key().unwrap().unwrap();
        assert_eq!(decoded, key);
    }

    #[test]
    fn test_security_config_decode_key_wrong_length() {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);

        let config = SecurityConfig {
            storage_encryption_key_b64: Some(b64),
            ..Default::default()
        };

        let result = config.decode_encryption_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();

        // Verify all subsystems are initialized
        // SecureKeyStorage.stats() is now synchronous
        assert!(manager.secure_storage().stats().encryption_algorithm == "none");
        assert!(manager.rate_limiter().config().enabled);
    }

    #[tokio::test]
    async fn test_security_manager_with_custom_sink() {
        let sink = Arc::new(MemoryAuditSink::new(100));
        let config = SecurityConfig::default();
        let manager = SecurityManager::with_audit_sink(sink.clone(), config).unwrap();

        // Store a key — should produce audit event (now synchronous)
        manager
            .secure_storage()
            .store_key("key-1", "signing", b"data", None)
            .unwrap();

        // Verify audit event was captured
        let events = sink.get_events_by_type("key_generated").await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_security_manager_rate_limiting() {
        let sink = Arc::new(MemoryAuditSink::new(100));
        let config = SecurityConfig {
            rate_limit: RateLimitConfig {
                requests_per_minute: 2,
                requests_per_hour: 100,
                requests_per_day: 1000,
                burst_size: 0,
                enabled: true,
            },
            ..Default::default()
        };
        let manager = SecurityManager::with_audit_sink(sink.clone(), config).unwrap();

        let key = RateLimitKey::Did("did:nexa:test".to_string());

        // First two should succeed — now synchronous
        assert!(manager.rate_limiter().check(&key).unwrap().is_allowed());
        assert!(manager.rate_limiter().check(&key).unwrap().is_allowed());

        // Third should fail and produce audit event
        let result = manager.rate_limiter().check(&key).unwrap();
        assert!(!result.is_allowed());

        let events = sink.get_events_by_type("rate_limit_exceeded").await;
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_security_manager_encryption() {
        use base64::Engine;
        let key = [42u8; 32];
        let b64 = base64::engine::general_purpose::STANDARD.encode(key);

        let config = SecurityConfig {
            storage_encryption_key_b64: Some(b64),
            ..Default::default()
        };

        let sink = Arc::new(MemoryAuditSink::new(100));
        let manager = SecurityManager::with_audit_sink(sink.clone(), config).unwrap();

        // Verify AES-256-GCM encryption is active (synchronous stats)
        let stats = manager.secure_storage().stats();
        assert!(stats.encryption_enabled);
        assert_eq!(stats.encryption_algorithm, "AES-256-GCM");

        // Store and retrieve key data (now synchronous)
        manager
            .secure_storage()
            .store_key("key-1", "signing", b"secret-data", None)
            .unwrap();

        let (data, _) = manager.secure_storage().get_key("key-1").unwrap().unwrap();
        assert_eq!(data, b"secret-data".to_vec());
    }
}
