//! In-Memory Storage Implementation
//!
//! Default storage backend using in-memory data structures.
//! Suitable for testing and single-node deployments without persistence requirements.

use super::{StorageError, StorageResult};
use crate::economy::{Channel, MicroReceipt};
use crate::types::{CapabilitySchema, Did};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory store configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum number of capabilities to store
    pub max_capabilities: usize,
    /// Maximum number of channels to store
    pub max_channels: usize,
    /// Maximum number of receipts to store
    pub max_receipts: usize,
    /// TTL for cached items (seconds)
    pub cache_ttl_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_capabilities: 10000,
            max_channels: 1000,
            max_receipts: 100000,
            cache_ttl_seconds: 300,
        }
    }
}

/// Stored capability with metadata
#[derive(Debug, Clone)]
pub struct StoredCapability {
    pub schema: CapabilitySchema,
    pub quality: serde_json::Value,
    pub available: bool,
    pub registered_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stored channel with metadata
#[derive(Debug, Clone)]
pub struct StoredChannel {
    pub channel: Channel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stored receipt
#[derive(Debug, Clone)]
pub struct StoredReceipt {
    pub receipt: MicroReceipt,
    pub created_at: DateTime<Utc>,
}

/// In-memory storage backend
pub struct MemoryStore {
    config: MemoryConfig,
    capabilities: Arc<RwLock<HashMap<String, StoredCapability>>>,
    channels: Arc<RwLock<HashMap<String, StoredChannel>>>,
    receipts: Arc<RwLock<Vec<StoredReceipt>>>,
    cache: Arc<RwLock<HashMap<String, (serde_json::Value, DateTime<Utc>)>>>,
}

impl MemoryStore {
    /// Create a new memory store
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            receipts: Arc::new(RwLock::new(Vec::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn default_store() -> Self {
        Self::new(MemoryConfig::default())
    }
}

// Capability storage operations
impl MemoryStore {
    /// Register a capability
    pub async fn register_capability(&self, schema: CapabilitySchema) -> StorageResult<()> {
        let mut caps = self.capabilities.write().await;

        let did_str = schema.metadata.did.to_string();

        if caps.contains_key(&did_str) {
            return Err(StorageError::Conflict(format!(
                "Capability already registered: {}",
                did_str
            )));
        }

        // Check capacity
        if caps.len() >= self.config.max_capabilities {
            // Remove oldest entry
            if let Some(oldest_key) = caps
                .iter()
                .min_by_key(|(_, c)| c.registered_at)
                .map(|(k, _)| k.clone())
            {
                caps.remove(&oldest_key);
            }
        }

        let now = Utc::now();
        caps.insert(
            did_str,
            StoredCapability {
                schema,
                quality: serde_json::json!({}),
                available: true,
                registered_at: now,
                updated_at: now,
            },
        );

        Ok(())
    }

    /// Unregister a capability
    pub async fn unregister_capability(&self, did: &str) -> StorageResult<()> {
        let mut caps = self.capabilities.write().await;

        if caps.remove(did).is_none() {
            return Err(StorageError::NotFound(format!(
                "Capability not found: {}",
                did
            )));
        }

        Ok(())
    }

    /// Get a capability by did
    pub async fn get_capability(&self, did: &str) -> StorageResult<Option<StoredCapability>> {
        let caps = self.capabilities.read().await;
        Ok(caps.get(did).cloned())
    }

    /// List all capabilities
    pub async fn list_capabilities(&self) -> StorageResult<Vec<StoredCapability>> {
        let caps = self.capabilities.read().await;
        Ok(caps.values().cloned().collect())
    }

    /// Update capability availability
    pub async fn set_capability_availability(
        &self,
        did: &str,
        available: bool,
    ) -> StorageResult<()> {
        let mut caps = self.capabilities.write().await;

        if let Some(cap) = caps.get_mut(did) {
            cap.available = available;
            cap.updated_at = Utc::now();
            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "Capability not found: {}",
                did
            )))
        }
    }

    /// Update capability quality metrics
    pub async fn update_capability_quality(
        &self,
        did: &str,
        quality: serde_json::Value,
    ) -> StorageResult<()> {
        let mut caps = self.capabilities.write().await;

        if let Some(cap) = caps.get_mut(did) {
            cap.quality = quality;
            cap.updated_at = Utc::now();
            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "Capability not found: {}",
                did
            )))
        }
    }

    /// Find capabilities by tags
    pub async fn find_capabilities_by_tags(
        &self,
        tags: &[String],
    ) -> StorageResult<Vec<StoredCapability>> {
        let caps = self.capabilities.read().await;

        Ok(caps
            .values()
            .filter(|c| tags.iter().all(|tag| c.schema.metadata.tags.contains(tag)))
            .cloned()
            .collect())
    }
}

// Channel storage operations
impl MemoryStore {
    /// Store a channel
    pub async fn store_channel(&self, channel: Channel) -> StorageResult<()> {
        let mut channels = self.channels.write().await;

        let channel_id = channel.id.clone();

        // Check capacity
        if channels.len() >= self.config.max_channels && !channels.contains_key(&channel_id) {
            // Remove closed channels first
            let closed_keys: Vec<_> = channels
                .iter()
                .filter(|(_, c)| c.channel.is_closed())
                .map(|(k, _)| k.clone())
                .collect();

            for key in closed_keys {
                channels.remove(&key);
            }

            // If still at capacity, remove oldest
            if channels.len() >= self.config.max_channels {
                if let Some(oldest_key) = channels
                    .iter()
                    .min_by_key(|(_, c)| c.created_at)
                    .map(|(k, _)| k.clone())
                {
                    channels.remove(&oldest_key);
                }
            }
        }

        let now = Utc::now();
        channels.insert(
            channel_id,
            StoredChannel {
                channel,
                created_at: now,
                updated_at: now,
            },
        );

        Ok(())
    }

    /// Get a channel by id
    pub async fn get_channel(&self, channel_id: &str) -> StorageResult<Option<StoredChannel>> {
        let channels = self.channels.read().await;
        Ok(channels.get(channel_id).cloned())
    }

    /// Update a channel
    pub async fn update_channel(&self, channel: Channel) -> StorageResult<()> {
        let mut channels = self.channels.write().await;

        let channel_id = channel.id.clone();
        if let Some(stored) = channels.get_mut(&channel_id) {
            stored.channel = channel;
            stored.updated_at = Utc::now();
            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "Channel not found: {}",
                channel_id
            )))
        }
    }

    /// List all open channels
    pub async fn list_open_channels(&self) -> StorageResult<Vec<StoredChannel>> {
        let channels = self.channels.read().await;
        Ok(channels
            .values()
            .filter(|c| !c.channel.is_closed())
            .cloned()
            .collect())
    }

    /// List channels for a peer
    pub async fn list_channels_for_peer(&self, did: &Did) -> StorageResult<Vec<StoredChannel>> {
        let channels = self.channels.read().await;
        Ok(channels
            .values()
            .filter(|c| c.channel.party_a == *did || c.channel.party_b == *did)
            .cloned()
            .collect())
    }

    /// Remove a channel
    pub async fn remove_channel(&self, channel_id: &str) -> StorageResult<()> {
        let mut channels = self.channels.write().await;

        if channels.remove(channel_id).is_none() {
            return Err(StorageError::NotFound(format!(
                "Channel not found: {}",
                channel_id
            )));
        }

        Ok(())
    }
}

// Receipt storage operations
impl MemoryStore {
    /// Store a receipt
    pub async fn store_receipt(&self, receipt: MicroReceipt) -> StorageResult<()> {
        let mut receipts = self.receipts.write().await;

        // Check capacity
        if receipts.len() >= self.config.max_receipts {
            // Remove oldest receipts
            receipts.sort_by_key(|r| r.created_at);
            let remove_count = receipts.len() / 10; // Remove 10%
            receipts.drain(0..remove_count);
        }

        receipts.push(StoredReceipt {
            receipt,
            created_at: Utc::now(),
        });

        Ok(())
    }

    /// Get receipts for a payer
    pub async fn get_receipts_for_payer(
        &self,
        payer_did: &Did,
    ) -> StorageResult<Vec<StoredReceipt>> {
        let receipts = self.receipts.read().await;
        let payer_str = payer_did.to_string();
        Ok(receipts
            .iter()
            .filter(|r| r.receipt.payer == payer_str)
            .cloned()
            .collect())
    }

    /// Get receipts for a payee
    pub async fn get_receipts_for_payee(
        &self,
        payee_did: &Did,
    ) -> StorageResult<Vec<StoredReceipt>> {
        let receipts = self.receipts.read().await;
        let payee_str = payee_did.to_string();
        Ok(receipts
            .iter()
            .filter(|r| r.receipt.payee == payee_str)
            .cloned()
            .collect())
    }

    /// Get receipts for a call
    pub async fn get_receipts_for_call(&self, call_id: &str) -> StorageResult<Vec<StoredReceipt>> {
        let receipts = self.receipts.read().await;
        Ok(receipts
            .iter()
            .filter(|r| r.receipt.call_id == call_id)
            .cloned()
            .collect())
    }
}

// Cache operations
impl MemoryStore {
    /// Set a cached value
    pub async fn cache_set(&self, key: &str, value: serde_json::Value) -> StorageResult<()> {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), (value, Utc::now()));
        Ok(())
    }

    /// Get a cached value
    pub async fn cache_get(&self, key: &str) -> StorageResult<Option<serde_json::Value>> {
        let cache = self.cache.read().await;

        if let Some((value, timestamp)) = cache.get(key) {
            // Check TTL
            let elapsed = (Utc::now() - *timestamp).num_seconds() as u64;
            if elapsed < self.config.cache_ttl_seconds {
                return Ok(Some(value.clone()));
            }
        }

        Ok(None)
    }

    /// Delete a cached value
    pub async fn cache_delete(&self, key: &str) -> StorageResult<()> {
        let mut cache = self.cache.write().await;
        cache.remove(key);
        Ok(())
    }

    /// Clear expired cache entries
    pub async fn cache_cleanup(&self) -> StorageResult<usize> {
        let mut cache = self.cache.write().await;
        let now = Utc::now();
        let ttl = self.config.cache_ttl_seconds as i64;

        let expired: Vec<_> = cache
            .iter()
            .filter(|(_, (_, ts))| (now - *ts).num_seconds() > ttl)
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired.len();
        for key in expired {
            cache.remove(&key);
        }

        Ok(count)
    }
}

// Statistics
impl MemoryStore {
    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        let caps = self.capabilities.read().await;
        let channels = self.channels.read().await;
        let receipts = self.receipts.read().await;
        let cache = self.cache.read().await;

        StorageStats {
            capabilities_count: caps.len(),
            channels_count: channels.len(),
            receipts_count: receipts.len(),
            cache_entries: cache.len(),
            max_capabilities: self.config.max_capabilities,
            max_channels: self.config.max_channels,
            max_receipts: self.config.max_receipts,
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub capabilities_count: usize,
    pub channels_count: usize,
    pub receipts_count: usize,
    pub cache_entries: usize,
    pub max_capabilities: usize,
    pub max_channels: usize,
    pub max_receipts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EndpointDefinition, ServiceMetadata};

    fn create_test_schema(did: &str, name: &str) -> CapabilitySchema {
        CapabilitySchema {
            version: "1.0".to_string(),
            metadata: ServiceMetadata {
                did: Did::new(did),
                name: name.to_string(),
                description: "Test service".to_string(),
                tags: vec!["test".to_string()],
            },
            endpoints: vec![],
        }
    }

    #[tokio::test]
    async fn test_capability_storage() {
        let store = MemoryStore::default_store();

        let did = Did::new("did:nexa:test123");
        let schema = create_test_schema("did:nexa:test123", "test-service");

        // Register
        store.register_capability(schema.clone()).await.unwrap();

        // Get
        let cap = store.get_capability(&did.to_string()).await.unwrap();
        assert!(cap.is_some());
        assert_eq!(cap.unwrap().schema.metadata.name, "test-service");

        // List
        let caps = store.list_capabilities().await.unwrap();
        assert_eq!(caps.len(), 1);

        // Unregister
        store.unregister_capability(&did.to_string()).await.unwrap();
        let cap = store.get_capability(&did.to_string()).await.unwrap();
        assert!(cap.is_none());
    }

    #[tokio::test]
    async fn test_channel_storage() {
        let store = MemoryStore::default_store();

        let party_a = Did::new("did:nexa:party_a");
        let party_b = Did::new("did:nexa:party_b");
        let channel = Channel::new("channel-1", party_a, party_b, 1000, 500);

        // Store
        store.store_channel(channel.clone()).await.unwrap();

        // Get
        let stored = store.get_channel("channel-1").await.unwrap();
        assert!(stored.is_some());

        // List open
        let open = store.list_open_channels().await.unwrap();
        assert_eq!(open.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let store = MemoryStore::default_store();

        let value = serde_json::json!({"key": "value"});

        // Set
        store.cache_set("test-key", value.clone()).await.unwrap();

        // Get
        let cached = store.cache_get("test-key").await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), value);

        // Delete
        store.cache_delete("test-key").await.unwrap();
        let cached = store.cache_get("test-key").await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let store = MemoryStore::default_store();

        let stats = store.stats().await;
        assert_eq!(stats.capabilities_count, 0);
        assert_eq!(stats.channels_count, 0);
        assert_eq!(stats.receipts_count, 0);
    }
}
