//! Storage Module
//!
//! Provides persistent storage backends for Nexa-net data.
//! Supports PostgreSQL for durable storage and Redis for caching.

#[cfg(feature = "storage-postgres")]
pub mod postgres;

#[cfg(feature = "storage-redis")]
pub mod redis;

pub mod memory;

// Re-export common types
pub use memory::{MemoryConfig, MemoryStore};

#[cfg(feature = "storage-postgres")]
pub use postgres::{PostgresConfig, PostgresStore};

#[cfg(feature = "storage-redis")]
pub use redis::{RedisConfig, RedisStore};

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// PostgreSQL connection URL (if enabled)
    #[cfg(feature = "storage-postgres")]
    pub postgres_url: Option<String>,

    /// Redis connection URL (if enabled)
    #[cfg(feature = "storage-redis")]
    pub redis_url: Option<String>,

    /// Memory store configuration (fallback)
    pub memory: MemoryConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "storage-postgres")]
            postgres_url: None,

            #[cfg(feature = "storage-redis")]
            redis_url: None,

            memory: MemoryConfig::default(),
        }
    }
}

/// Generic storage error
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<StorageError> for crate::error::Error {
    fn from(e: StorageError) -> Self {
        crate::error::Error::Internal(e.to_string())
    }
}

/// Result type for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;
