//! Storage Module
//!
//! Provides persistent storage backends for Nexa-net data.
//!
//! # Architecture
//!
//! All storage backends implement the unified `Storage` trait, enabling
//! seamless backend switching without changing business logic.
//!
//! # Backends
//!
//! - **MemoryStore**: In-memory storage (default, for testing/single-node)
//! - **RocksDBStore**: Persistent key-value storage (feature: `storage-rocksdb`)
//! - **PostgresStore**: PostgreSQL durable storage (feature: `storage-postgres`)
//! - **RedisStore**: Redis caching layer (feature: `storage-redis`)

pub mod memory;

#[cfg(feature = "storage-rocksdb")]
pub mod rocksdb;

#[cfg(feature = "storage-postgres")]
pub mod postgres;

#[cfg(feature = "storage-redis")]
pub mod redis;

// Re-export common types
pub use memory::{MemoryConfig, MemoryStore, StoredCapability, StoredChannel, StoredReceipt};

#[cfg(feature = "storage-rocksdb")]
pub use rocksdb::{RocksDBConfig, RocksDBStore};

#[cfg(feature = "storage-postgres")]
pub use postgres::{PostgresConfig, PostgresStore};

#[cfg(feature = "storage-redis")]
pub use redis::{RedisConfig, RedisStore};

use crate::economy::{Channel, MicroReceipt};
use crate::types::{CapabilitySchema, Did};
use async_trait::async_trait;

// ============================================================================
// Unified Storage Trait
// ============================================================================

/// Unified storage trait that all backends must implement.
///
/// This trait defines the complete interface for Nexa-net persistent storage,
/// covering capabilities, channels, receipts, and caching.
///
/// # Backend Selection
///
/// ```rust,ignore
/// // Memory backend (default, no persistence)
/// let store = MemoryStore::default_store();
///
/// // RocksDB backend (persistent, requires feature)
/// let store = RocksDBStore::new(RocksDBConfig::default());
///
/// // Use interchangeably via Storage trait
/// let dyn_store: Arc<dyn Storage> = Arc::new(store);
/// ```
#[async_trait]
pub trait Storage: Send + Sync {
    // ========================================================================
    // Capability Operations
    // ========================================================================

    /// Register a new capability
    async fn register_capability(&self, schema: CapabilitySchema) -> StorageResult<()>;

    /// Unregister a capability by DID
    async fn unregister_capability(&self, did: &str) -> StorageResult<()>;

    /// Get a capability by DID
    async fn get_capability(&self, did: &str) -> StorageResult<Option<StoredCapability>>;

    /// List all registered capabilities
    async fn list_capabilities(&self) -> StorageResult<Vec<StoredCapability>>;

    /// Set capability availability
    async fn set_capability_availability(&self, did: &str, available: bool) -> StorageResult<()>;

    /// Update capability quality metrics
    async fn update_capability_quality(
        &self,
        did: &str,
        quality: serde_json::Value,
    ) -> StorageResult<()>;

    /// Find capabilities by tags
    async fn find_capabilities_by_tags(
        &self,
        tags: &[String],
    ) -> StorageResult<Vec<StoredCapability>>;

    // ========================================================================
    // Channel Operations
    // ========================================================================

    /// Store a channel
    async fn store_channel(&self, channel: Channel) -> StorageResult<()>;

    /// Get a channel by ID
    async fn get_channel(&self, channel_id: &str) -> StorageResult<Option<StoredChannel>>;

    /// Update a channel
    async fn update_channel(&self, channel: Channel) -> StorageResult<()>;

    /// List all open channels
    async fn list_open_channels(&self) -> StorageResult<Vec<StoredChannel>>;

    /// List channels for a specific peer
    async fn list_channels_for_peer(&self, did: &Did) -> StorageResult<Vec<StoredChannel>>;

    /// Remove a channel
    async fn remove_channel(&self, channel_id: &str) -> StorageResult<()>;

    // ========================================================================
    // Receipt Operations
    // ========================================================================

    /// Store a receipt
    async fn store_receipt(&self, receipt: MicroReceipt) -> StorageResult<()>;

    /// Get receipts for a payer DID
    async fn get_receipts_for_payer(&self, payer_did: &Did) -> StorageResult<Vec<StoredReceipt>>;

    /// Get receipts for a payee DID
    async fn get_receipts_for_payee(&self, payee_did: &Did) -> StorageResult<Vec<StoredReceipt>>;

    /// Get receipts for a specific call
    async fn get_receipts_for_call(&self, call_id: &str) -> StorageResult<Vec<StoredReceipt>>;

    // ========================================================================
    // Cache Operations
    // ========================================================================

    /// Set a cached value with TTL
    async fn cache_set(&self, key: &str, value: serde_json::Value) -> StorageResult<()>;

    /// Get a cached value (returns None if expired or missing)
    async fn cache_get(&self, key: &str) -> StorageResult<Option<serde_json::Value>>;

    /// Delete a cached value
    async fn cache_delete(&self, key: &str) -> StorageResult<()>;

    /// Clean up expired cache entries
    async fn cache_cleanup(&self) -> StorageResult<usize>;

    // ========================================================================
    // Stats
    // ========================================================================

    /// Get storage statistics
    async fn stats(&self) -> StorageStats;
}

// ============================================================================
// Configuration & Error Types
// ============================================================================

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Memory store configuration (always available as fallback)
    pub memory: MemoryConfig,

    /// RocksDB path (if enabled)
    #[cfg(feature = "storage-rocksdb")]
    pub rocksdb_path: Option<String>,

    /// PostgreSQL connection URL (if enabled)
    #[cfg(feature = "storage-postgres")]
    pub postgres_url: Option<String>,

    /// Redis connection URL (if enabled)
    #[cfg(feature = "storage-redis")]
    pub redis_url: Option<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            memory: MemoryConfig::default(),
            #[cfg(feature = "storage-rocksdb")]
            rocksdb_path: None,
            #[cfg(feature = "storage-postgres")]
            postgres_url: None,
            #[cfg(feature = "storage-redis")]
            redis_url: None,
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Number of stored capabilities
    pub capabilities_count: usize,
    /// Number of stored channels
    pub channels_count: usize,
    /// Number of stored receipts
    pub receipts_count: usize,
    /// Number of cache entries
    pub cache_entries: usize,
    /// Maximum capabilities (0 = no hard limit)
    pub max_capabilities: usize,
    /// Maximum channels (0 = no hard limit)
    pub max_channels: usize,
    /// Maximum receipts (0 = no hard limit)
    pub max_receipts: usize,
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

    #[error("Capacity exceeded: {0}")]
    Capacity(String),

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
