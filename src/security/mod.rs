//! Security Module
//!
//! Provides security enhancements for Nexa-net:
//! - Secure key storage with encryption
//! - Key rotation management
//! - Audit logging
//! - Rate limiting

pub mod audit;
pub mod key_rotation;
pub mod rate_limit;
pub mod secure_storage;

// Re-exports
pub use audit::{AuditEvent, AuditLogger, AuditSink};
pub use key_rotation::{KeyRotationPolicy, KeyRotator};
pub use rate_limit::{RateLimiter, RateLimitConfig};
pub use secure_storage::{SecureKeyStorage, KeyMetadata};

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable audit logging
    pub audit_enabled: bool,
    /// Key rotation policy
    pub key_rotation: KeyRotationPolicy,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// Encryption key for secure storage (should be loaded from secure source)
    pub storage_encryption_key: Option<[u8; 32]>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            audit_enabled: true,
            key_rotation: KeyRotationPolicy::default(),
            rate_limit: RateLimitConfig::default(),
            storage_encryption_key: None,
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