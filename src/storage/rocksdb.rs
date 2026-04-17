//! RocksDB Storage Backend
//!
//! Persistent key-value storage using RocksDB.
//! Implements the `Storage` trait for durable, high-performance persistence.
//!
//! # Column Families
//!
//! - `capabilities` — Service capability schemas
//! - `channels` — State channel data
//! - `receipts` — Micro-receipt records
//! - `cache` — Cached values with TTL metadata

use super::{
    Storage, StorageError, StorageResult, StorageStats, StoredCapability, StoredChannel,
    StoredReceipt,
};
use crate::economy::{Channel, MicroReceipt};
use crate::types::{CapabilitySchema, Did};
use async_trait::async_trait;
use chrono::Utc;
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::sync::Arc;
use tokio::sync::RwLock;

/// RocksDB storage configuration
#[derive(Debug, Clone)]
pub struct RocksDBConfig {
    /// Database path on disk
    pub path: String,
    /// Whether to create database if it doesn't exist
    pub create_if_missing: bool,
    /// Maximum number of open files
    pub max_open_files: i32,
    /// Write buffer size in bytes (per column family)
    pub write_buffer_size: usize,
    /// Maximum total write buffer size across all column families
    pub max_write_buffer_number: i32,
}

impl Default for RocksDBConfig {
    fn default() -> Self {
        Self {
            path: "./data/nexa-net-rocksdb".to_string(),
            create_if_missing: true,
            max_open_files: 256,
            write_buffer_size: 64 * 1024 * 1024, // 64 MB
            max_write_buffer_number: 4,
        }
    }
}

// Column family names
const CF_CAPABILITIES: &str = "capabilities";
const CF_CHANNELS: &str = "channels";
const CF_RECEIPTS: &str = "receipts";
const CF_CACHE: &str = "cache";

/// RocksDB storage backend implementing `Storage` trait
///
/// Uses RocksDB for persistence while maintaining in-memory indexes
/// for fast query operations (tag-based search, peer filtering, etc.)
#[allow(clippy::type_complexity)]
pub struct RocksDBStore {
    #[allow(dead_code)]
    config: RocksDBConfig,
    db: Arc<DB>,
    // In-memory indexes for fast queries (supplement RocksDB KV store)
    capabilities_index: Arc<RwLock<std::collections::HashMap<String, StoredCapability>>>,
    channels_index: Arc<RwLock<std::collections::HashMap<String, StoredChannel>>>,
    receipts_list: Arc<RwLock<Vec<StoredReceipt>>>,
    cache_index:
        Arc<RwLock<std::collections::HashMap<String, (serde_json::Value, chrono::DateTime<Utc>)>>>,
    cache_ttl_seconds: u64,
}

impl RocksDBStore {
    /// Create a new RocksDB store
    pub fn new(config: RocksDBConfig) -> StorageResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        opts.set_max_open_files(config.max_open_files);
        opts.set_write_buffer_size(config.write_buffer_size);
        opts.set_max_write_buffer_number(config.max_write_buffer_number);

        // Define column families
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new(CF_CAPABILITIES, Options::default()),
            ColumnFamilyDescriptor::new(CF_CHANNELS, Options::default()),
            ColumnFamilyDescriptor::new(CF_RECEIPTS, Options::default()),
            ColumnFamilyDescriptor::new(CF_CACHE, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, &config.path, cf_descriptors).map_err(|e| {
            StorageError::Connection(format!(
                "Failed to open RocksDB at '{}': {}",
                config.path, e
            ))
        })?;

        Ok(Self {
            config,
            db: Arc::new(db),
            capabilities_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            channels_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            receipts_list: Arc::new(RwLock::new(Vec::new())),
            cache_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            cache_ttl_seconds: 300,
        })
    }

    /// Create with default configuration
    pub fn default_store() -> StorageResult<Self> {
        Self::new(RocksDBConfig::default())
    }

    /// Helper: get column family handle
    fn get_cf(&self, name: &str) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| StorageError::Internal(format!("Column family '{}' not found", name)))
    }

    /// Helper: serialize value to bytes
    fn to_bytes<T: serde::Serialize>(value: &T) -> StorageResult<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| StorageError::Serialization(e.to_string()))
    }

    #[allow(dead_code)]
    /// Helper: deserialize bytes to value
    fn from_bytes<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> StorageResult<T> {
        serde_json::from_slice(bytes).map_err(|e| StorageError::Serialization(e.to_string()))
    }

    /// Flush pending writes to disk
    pub fn flush(&self) -> StorageResult<()> {
        self.db
            .flush()
            .map_err(|e| StorageError::Internal(format!("RocksDB flush error: {}", e)))?;
        Ok(())
    }
}

// ============================================================================
// Storage Trait Implementation
// ============================================================================

#[async_trait]
impl Storage for RocksDBStore {
    async fn register_capability(&self, schema: CapabilitySchema) -> StorageResult<()> {
        let did_str = schema.metadata.did.to_string();

        // Check for conflict in memory index
        {
            let idx = self.capabilities_index.read().await;
            if idx.contains_key(&did_str) {
                return Err(StorageError::Conflict(format!(
                    "Capability already registered: {}",
                    did_str
                )));
            }
        }

        let stored = StoredCapability {
            schema,
            quality: serde_json::json!({}),
            available: true,
            registered_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Persist to RocksDB
        let cf = self.get_cf(CF_CAPABILITIES)?;
        let value = RocksDBStore::to_bytes(&stored)?;
        self.db
            .put_cf(cf, did_str.as_bytes(), &value)
            .map_err(|e| StorageError::Query(e.to_string()))?;

        // Update in-memory index
        self.capabilities_index
            .write()
            .await
            .insert(did_str, stored);

        Ok(())
    }

    async fn unregister_capability(&self, did: &str) -> StorageResult<()> {
        let cf = self.get_cf(CF_CAPABILITIES)?;
        self.db
            .delete_cf(cf, did.as_bytes())
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.capabilities_index.write().await.remove(did);

        Ok(())
    }

    async fn get_capability(&self, did: &str) -> StorageResult<Option<StoredCapability>> {
        let idx = self.capabilities_index.read().await;
        Ok(idx.get(did).cloned())
    }

    async fn list_capabilities(&self) -> StorageResult<Vec<StoredCapability>> {
        let idx = self.capabilities_index.read().await;
        Ok(idx.values().cloned().collect())
    }

    async fn set_capability_availability(&self, did: &str, available: bool) -> StorageResult<()> {
        let mut idx = self.capabilities_index.write().await;
        if let Some(cap) = idx.get_mut(did) {
            cap.available = available;
            cap.updated_at = Utc::now();

            // Persist updated entry
            let cf = self.get_cf(CF_CAPABILITIES)?;
            let value = RocksDBStore::to_bytes(cap)?;
            self.db
                .put_cf(cf, did.as_bytes(), &value)
                .map_err(|e| StorageError::Query(e.to_string()))?;

            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "Capability not found: {}",
                did
            )))
        }
    }

    async fn update_capability_quality(
        &self,
        did: &str,
        quality: serde_json::Value,
    ) -> StorageResult<()> {
        let mut idx = self.capabilities_index.write().await;
        if let Some(cap) = idx.get_mut(did) {
            cap.quality = quality;
            cap.updated_at = Utc::now();

            let cf = self.get_cf(CF_CAPABILITIES)?;
            let value = RocksDBStore::to_bytes(cap)?;
            self.db
                .put_cf(cf, did.as_bytes(), &value)
                .map_err(|e| StorageError::Query(e.to_string()))?;

            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "Capability not found: {}",
                did
            )))
        }
    }

    async fn find_capabilities_by_tags(
        &self,
        tags: &[String],
    ) -> StorageResult<Vec<StoredCapability>> {
        let idx = self.capabilities_index.read().await;
        Ok(idx
            .values()
            .filter(|c| tags.iter().all(|tag| c.schema.metadata.tags.contains(tag)))
            .cloned()
            .collect())
    }

    async fn store_channel(&self, channel: Channel) -> StorageResult<()> {
        let channel_id = channel.id.clone();

        let stored = StoredChannel {
            channel,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cf = self.get_cf(CF_CHANNELS)?;
        let value = RocksDBStore::to_bytes(&stored)?;
        self.db
            .put_cf(cf, channel_id.as_bytes(), &value)
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.channels_index.write().await.insert(channel_id, stored);

        Ok(())
    }

    async fn get_channel(&self, channel_id: &str) -> StorageResult<Option<StoredChannel>> {
        let idx = self.channels_index.read().await;
        Ok(idx.get(channel_id).cloned())
    }

    async fn update_channel(&self, channel: Channel) -> StorageResult<()> {
        let channel_id = channel.id.clone();
        let mut idx = self.channels_index.write().await;

        if let Some(stored) = idx.get_mut(&channel_id) {
            stored.channel = channel;
            stored.updated_at = Utc::now();

            let cf = self.get_cf(CF_CHANNELS)?;
            let value = RocksDBStore::to_bytes(stored)?;
            self.db
                .put_cf(cf, channel_id.as_bytes(), &value)
                .map_err(|e| StorageError::Query(e.to_string()))?;

            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "Channel not found: {}",
                channel_id
            )))
        }
    }

    async fn list_open_channels(&self) -> StorageResult<Vec<StoredChannel>> {
        let idx = self.channels_index.read().await;
        Ok(idx
            .values()
            .filter(|c| !c.channel.is_closed())
            .cloned()
            .collect())
    }

    async fn list_channels_for_peer(&self, did: &Did) -> StorageResult<Vec<StoredChannel>> {
        let idx = self.channels_index.read().await;
        Ok(idx
            .values()
            .filter(|c| c.channel.party_a == *did || c.channel.party_b == *did)
            .cloned()
            .collect())
    }

    async fn remove_channel(&self, channel_id: &str) -> StorageResult<()> {
        let cf = self.get_cf(CF_CHANNELS)?;
        self.db
            .delete_cf(cf, channel_id.as_bytes())
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.channels_index.write().await.remove(channel_id);

        Ok(())
    }

    async fn store_receipt(&self, receipt: MicroReceipt) -> StorageResult<()> {
        let stored = StoredReceipt {
            receipt,
            created_at: Utc::now(),
        };

        let receipt_id = stored.receipt.receipt_id.clone();
        let cf = self.get_cf(CF_RECEIPTS)?;
        let value = RocksDBStore::to_bytes(&stored)?;
        self.db
            .put_cf(cf, receipt_id.as_bytes(), &value)
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.receipts_list.write().await.push(stored);

        Ok(())
    }

    async fn get_receipts_for_payer(&self, payer_did: &Did) -> StorageResult<Vec<StoredReceipt>> {
        let list = self.receipts_list.read().await;
        let payer_str = payer_did.to_string();
        Ok(list
            .iter()
            .filter(|r| r.receipt.payer == payer_str)
            .cloned()
            .collect())
    }

    async fn get_receipts_for_payee(&self, payee_did: &Did) -> StorageResult<Vec<StoredReceipt>> {
        let list = self.receipts_list.read().await;
        let payee_str = payee_did.to_string();
        Ok(list
            .iter()
            .filter(|r| r.receipt.payee == payee_str)
            .cloned()
            .collect())
    }

    async fn get_receipts_for_call(&self, call_id: &str) -> StorageResult<Vec<StoredReceipt>> {
        let list = self.receipts_list.read().await;
        Ok(list
            .iter()
            .filter(|r| r.receipt.call_id == call_id)
            .cloned()
            .collect())
    }

    async fn cache_set(&self, key: &str, value: serde_json::Value) -> StorageResult<()> {
        let now = Utc::now();

        let cf = self.get_cf(CF_CACHE)?;
        let cache_entry = serde_json::json!({
            "value": value,
            "timestamp": now.to_rfc3339(),
        });
        let serialized = RocksDBStore::to_bytes(&cache_entry)?;
        self.db
            .put_cf(cf, key.as_bytes(), &serialized)
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.cache_index
            .write()
            .await
            .insert(key.to_string(), (value, now));

        Ok(())
    }

    async fn cache_get(&self, key: &str) -> StorageResult<Option<serde_json::Value>> {
        let idx = self.cache_index.read().await;
        if let Some((value, timestamp)) = idx.get(key) {
            let elapsed = (Utc::now() - *timestamp).num_seconds() as u64;
            if elapsed < self.cache_ttl_seconds {
                return Ok(Some(value.clone()));
            }
        }
        Ok(None)
    }

    async fn cache_delete(&self, key: &str) -> StorageResult<()> {
        let cf = self.get_cf(CF_CACHE)?;
        self.db
            .delete_cf(cf, key.as_bytes())
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.cache_index.write().await.remove(key);

        Ok(())
    }

    async fn cache_cleanup(&self) -> StorageResult<usize> {
        let mut idx = self.cache_index.write().await;
        let now = Utc::now();
        let ttl = self.cache_ttl_seconds as i64;

        let expired_keys: Vec<String> = idx
            .iter()
            .filter(|(_, (_, ts))| (now - *ts).num_seconds() > ttl)
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired_keys.len();

        // Remove from RocksDB
        let cf = self.get_cf(CF_CACHE)?;
        let mut batch = WriteBatch::default();
        for key in &expired_keys {
            batch.delete_cf(cf, key.as_bytes());
        }
        self.db
            .write(batch)
            .map_err(|e| StorageError::Query(e.to_string()))?;

        // Remove from in-memory index
        for key in expired_keys {
            idx.remove(&key);
        }

        Ok(count)
    }

    async fn stats(&self) -> StorageStats {
        let caps = self.capabilities_index.read().await;
        let channels = self.channels_index.read().await;
        let receipts = self.receipts_list.read().await;
        let cache = self.cache_index.read().await;

        StorageStats {
            capabilities_count: caps.len(),
            channels_count: channels.len(),
            receipts_count: receipts.len(),
            cache_entries: cache.len(),
            max_capabilities: 0, // RocksDB has no hard limit
            max_channels: 0,
            max_receipts: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> RocksDBConfig {
        let dir = TempDir::new().unwrap();
        RocksDBConfig {
            path: dir.path().to_str().unwrap().to_string(),
            create_if_missing: true,
            max_open_files: 64,
            write_buffer_size: 4 * 1024 * 1024,
            max_write_buffer_number: 2,
        }
    }

    #[tokio::test]
    async fn test_rocksdb_store_creation() {
        let config = create_test_config();
        let store = RocksDBStore::new(config).unwrap();
        let stats = store.stats().await;
        assert_eq!(stats.capabilities_count, 0);
    }

    #[tokio::test]
    async fn test_rocksdb_capability_roundtrip() {
        let config = create_test_config();
        let store = RocksDBStore::new(config).unwrap();

        let schema = CapabilitySchema {
            version: "1.0".to_string(),
            metadata: crate::types::ServiceMetadata {
                did: Did::new("did:nexa:test-rocksdb"),
                name: "test-service".to_string(),
                description: "Test service for RocksDB".to_string(),
                tags: vec!["test".to_string()],
            },
            endpoints: vec![],
        };

        // Register
        store.register_capability(schema.clone()).await.unwrap();

        // Get
        let cap = store.get_capability("did:nexa:test-rocksdb").await.unwrap();
        assert!(cap.is_some());
        assert_eq!(cap.unwrap().schema.metadata.name, "test-service");

        // Unregister
        store
            .unregister_capability("did:nexa:test-rocksdb")
            .await
            .unwrap();
        let cap = store.get_capability("did:nexa:test-rocksdb").await.unwrap();
        assert!(cap.is_none());
    }

    #[tokio::test]
    async fn test_rocksdb_cache_roundtrip() {
        let config = create_test_config();
        let store = RocksDBStore::new(config).unwrap();

        let value = serde_json::json!({"key": "value"});

        store.cache_set("test-key", value.clone()).await.unwrap();
        let cached = store.cache_get("test-key").await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), value);

        store.cache_delete("test-key").await.unwrap();
        let cached = store.cache_get("test-key").await.unwrap();
        assert!(cached.is_none());
    }
}
