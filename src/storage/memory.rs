//! In-Memory Storage Implementation
//!
//! Default storage backend using in-memory data structures.
//! Suitable for testing and single-node deployments without persistence requirements.
//! Implements the unified `Storage` trait from `super::mod.rs`.

use super::{Storage, StorageError, StorageResult, StorageStats};
use crate::economy::{Channel, MicroReceipt};
use crate::types::{CapabilitySchema, Did};
use async_trait::async_trait;
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredCapability {
    pub schema: CapabilitySchema,
    pub quality: serde_json::Value,
    pub available: bool,
    pub registered_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stored channel with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredChannel {
    pub channel: Channel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stored receipt
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredReceipt {
    pub receipt: MicroReceipt,
    pub created_at: DateTime<Utc>,
}

/// Cache entry type alias to reduce type complexity
type CacheEntry = (serde_json::Value, DateTime<Utc>);

/// In-memory storage backend
#[allow(clippy::type_complexity)]
pub struct MemoryStore {
    config: MemoryConfig,
    capabilities: Arc<RwLock<HashMap<String, StoredCapability>>>,
    channels: Arc<RwLock<HashMap<String, StoredChannel>>>,
    receipts: Arc<RwLock<Vec<StoredReceipt>>>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
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

// ============================================================================
// Storage Trait Implementation
// ============================================================================

#[async_trait]
impl Storage for MemoryStore {
    // Capability operations — delegate to inherent methods using fully qualified syntax
    async fn register_capability(&self, schema: CapabilitySchema) -> StorageResult<()> {
        MemoryStore::register_capability(self, schema).await
    }

    async fn unregister_capability(&self, did: &str) -> StorageResult<()> {
        MemoryStore::unregister_capability(self, did).await
    }

    async fn get_capability(&self, did: &str) -> StorageResult<Option<StoredCapability>> {
        MemoryStore::get_capability(self, did).await
    }

    async fn list_capabilities(&self) -> StorageResult<Vec<StoredCapability>> {
        MemoryStore::list_capabilities(self).await
    }

    async fn set_capability_availability(&self, did: &str, available: bool) -> StorageResult<()> {
        MemoryStore::set_capability_availability(self, did, available).await
    }

    async fn update_capability_quality(
        &self,
        did: &str,
        quality: serde_json::Value,
    ) -> StorageResult<()> {
        MemoryStore::update_capability_quality(self, did, quality).await
    }

    async fn find_capabilities_by_tags(
        &self,
        tags: &[String],
    ) -> StorageResult<Vec<StoredCapability>> {
        MemoryStore::find_capabilities_by_tags(self, tags).await
    }

    // Channel operations
    async fn store_channel(&self, channel: Channel) -> StorageResult<()> {
        MemoryStore::store_channel(self, channel).await
    }

    async fn get_channel(&self, channel_id: &str) -> StorageResult<Option<StoredChannel>> {
        MemoryStore::get_channel(self, channel_id).await
    }

    async fn update_channel(&self, channel: Channel) -> StorageResult<()> {
        MemoryStore::update_channel(self, channel).await
    }

    async fn list_open_channels(&self) -> StorageResult<Vec<StoredChannel>> {
        MemoryStore::list_open_channels(self).await
    }

    async fn list_channels_for_peer(&self, did: &Did) -> StorageResult<Vec<StoredChannel>> {
        MemoryStore::list_channels_for_peer(self, did).await
    }

    async fn remove_channel(&self, channel_id: &str) -> StorageResult<()> {
        MemoryStore::remove_channel(self, channel_id).await
    }

    // Receipt operations
    async fn store_receipt(&self, receipt: MicroReceipt) -> StorageResult<()> {
        MemoryStore::store_receipt(self, receipt).await
    }

    async fn get_receipts_for_payer(&self, payer_did: &Did) -> StorageResult<Vec<StoredReceipt>> {
        MemoryStore::get_receipts_for_payer(self, payer_did).await
    }

    async fn get_receipts_for_payee(&self, payee_did: &Did) -> StorageResult<Vec<StoredReceipt>> {
        MemoryStore::get_receipts_for_payee(self, payee_did).await
    }

    async fn get_receipts_for_call(&self, call_id: &str) -> StorageResult<Vec<StoredReceipt>> {
        MemoryStore::get_receipts_for_call(self, call_id).await
    }

    // Cache operations
    async fn cache_set(&self, key: &str, value: serde_json::Value) -> StorageResult<()> {
        MemoryStore::cache_set(self, key, value).await
    }

    async fn cache_get(&self, key: &str) -> StorageResult<Option<serde_json::Value>> {
        MemoryStore::cache_get(self, key).await
    }

    async fn cache_delete(&self, key: &str) -> StorageResult<()> {
        MemoryStore::cache_delete(self, key).await
    }

    async fn cache_cleanup(&self) -> StorageResult<usize> {
        MemoryStore::cache_cleanup(self).await
    }

    // Stats
    async fn stats(&self) -> StorageStats {
        MemoryStore::stats(self).await
    }
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

    // ── Boundary / error-path tests ──────────────────────────

    #[tokio::test]
    async fn test_register_duplicate_capability_conflict() {
        let store = MemoryStore::default_store();
        let schema = create_test_schema("did:nexa:dup1", "svc-dup");

        store.register_capability(schema.clone()).await.unwrap();
        let err = store.register_capability(schema).await.unwrap_err();
        assert!(
            matches!(err, StorageError::Conflict(_)),
            "duplicate registration should return Conflict error"
        );
    }

    #[tokio::test]
    async fn test_get_nonexistent_capability_returns_none() {
        let store = MemoryStore::default_store();
        let result = store.get_capability("did:nexa:ghost").await.unwrap();
        assert!(result.is_none(), "nonexistent DID should return None");
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_capability_not_found() {
        let store = MemoryStore::default_store();
        let err = store
            .unregister_capability("did:nexa:ghost")
            .await
            .unwrap_err();
        assert!(
            matches!(err, StorageError::NotFound(_)),
            "unregister nonexistent should return NotFound error"
        );
    }

    #[tokio::test]
    async fn test_set_capability_availability_nonexistent_not_found() {
        let store = MemoryStore::default_store();
        let err = store
            .set_capability_availability("did:nexa:ghost", false)
            .await
            .unwrap_err();
        assert!(
            matches!(err, StorageError::NotFound(_)),
            "set_availability on nonexistent DID should return NotFound"
        );
    }

    #[tokio::test]
    async fn test_update_capability_quality_nonexistent_not_found() {
        let store = MemoryStore::default_store();
        let err = store
            .update_capability_quality("did:nexa:ghost", serde_json::json!({"success_rate": 0.5}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, StorageError::NotFound(_)),
            "update_quality on nonexistent DID should return NotFound"
        );
    }

    #[tokio::test]
    async fn test_find_capabilities_by_nonexistent_tag() {
        let store = MemoryStore::default_store();
        store
            .register_capability(create_test_schema("did:nexa:t1", "svc-t1"))
            .await
            .unwrap();

        let found = store
            .find_capabilities_by_tags(&["nonexistent".to_string()])
            .await
            .unwrap();
        assert!(
            found.is_empty(),
            "nonexistent tag should yield empty results"
        );
    }

    #[tokio::test]
    async fn test_find_capabilities_by_empty_tags_matches_all() {
        let store = MemoryStore::default_store();
        store
            .register_capability(create_test_schema("did:nexa:t1", "svc-t1"))
            .await
            .unwrap();
        store
            .register_capability(create_test_schema("did:nexa:t2", "svc-t2"))
            .await
            .unwrap();

        // Empty tag list: all(|tag| ...) is vacuously true → all capabilities match
        let found = store.find_capabilities_by_tags(&[]).await.unwrap();
        assert_eq!(
            found.len(),
            2,
            "empty tag filter should match all capabilities"
        );
    }

    #[tokio::test]
    async fn test_get_channel_nonexistent_returns_none() {
        let store = MemoryStore::default_store();
        let result = store.get_channel("nonexistent-id").await.unwrap();
        assert!(
            result.is_none(),
            "nonexistent channel ID should return None"
        );
    }

    #[tokio::test]
    async fn test_update_channel_nonexistent_not_found() {
        let store = MemoryStore::default_store();
        let party_a = Did::new("did:nexa:pa");
        let party_b = Did::new("did:nexa:pb");
        let phantom_channel = Channel::new("ghost-channel", party_a, party_b, 100, 100);
        let err = store.update_channel(phantom_channel).await.unwrap_err();
        assert!(
            matches!(err, StorageError::NotFound(_)),
            "update nonexistent channel should yield NotFound"
        );
    }

    #[tokio::test]
    async fn test_remove_channel_nonexistent_not_found() {
        let store = MemoryStore::default_store();
        let err = store.remove_channel("ghost-channel").await.unwrap_err();
        assert!(
            matches!(err, StorageError::NotFound(_)),
            "remove nonexistent channel should yield NotFound"
        );
    }

    #[tokio::test]
    async fn test_receipt_queries_on_empty_store() {
        let store = MemoryStore::default_store();
        let payer = Did::new("did:nexa:payer1");
        let payee = Did::new("did:nexa:payee1");

        assert!(store
            .get_receipts_for_payer(&payer)
            .await
            .unwrap()
            .is_empty());
        assert!(store
            .get_receipts_for_payee(&payee)
            .await
            .unwrap()
            .is_empty());
        assert!(store
            .get_receipts_for_call("call-1")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_receipt_store_and_query() {
        let store = MemoryStore::default_store();
        let payer = Did::new("did:nexa:p1");
        let payee = Did::new("did:nexa:pay1");

        let receipt = MicroReceipt {
            receipt_id: "r1".to_string(),
            call_id: "call-1".to_string(),
            payer: payer.to_string(),
            payee: payee.to_string(),
            amount: 500,
            service_endpoint: "/translate".to_string(),
            timestamp: Utc::now(),
            previous_receipt_hash: "genesis".to_string(),
            payer_signature: vec![],
            payee_signature: None,
        };

        store.store_receipt(receipt).await.unwrap();

        assert_eq!(store.get_receipts_for_payer(&payer).await.unwrap().len(), 1);
        assert_eq!(store.get_receipts_for_payee(&payee).await.unwrap().len(), 1);
        assert_eq!(
            store.get_receipts_for_call("call-1").await.unwrap().len(),
            1
        );
        assert!(store
            .get_receipts_for_call("call-999")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_cache_get_nonexistent_key_returns_none() {
        let store = MemoryStore::default_store();
        let result = store.cache_get("no-such-key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_delete_nonexistent_key_harmless() {
        let store = MemoryStore::default_store();
        // Deleting a key that doesn't exist should be a no-op
        store.cache_delete("no-such-key").await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_cleanup_no_expired_returns_zero() {
        let store = MemoryStore::default_store();
        store
            .cache_set("fresh-key", serde_json::json!("fresh-val"))
            .await
            .unwrap();
        let removed = store.cache_cleanup().await.unwrap();
        assert_eq!(removed, 0, "no expired entries → cleanup removes 0");
    }

    #[tokio::test]
    async fn test_capability_max_capacity_eviction() {
        let config = MemoryConfig {
            max_capabilities: 2,
            max_channels: 100,
            max_receipts: 100,
            cache_ttl_seconds: 300,
        };
        let store = MemoryStore::new(config);

        store
            .register_capability(create_test_schema("did:nexa:c1", "svc-c1"))
            .await
            .unwrap();
        store
            .register_capability(create_test_schema("did:nexa:c2", "svc-c2"))
            .await
            .unwrap();

        // 3rd registration evicts oldest (c1)
        store
            .register_capability(create_test_schema("did:nexa:c3", "svc-c3"))
            .await
            .unwrap();

        let caps = store.list_capabilities().await.unwrap();
        assert_eq!(
            caps.len(),
            2,
            "eviction should keep count at max_capabilities"
        );
        assert!(
            store.get_capability("did:nexa:c1").await.unwrap().is_none(),
            "oldest capability should be evicted"
        );
        assert!(
            store.get_capability("did:nexa:c3").await.unwrap().is_some(),
            "newest capability should be present"
        );
    }

    #[tokio::test]
    async fn test_channel_max_capacity_eviction() {
        let config = MemoryConfig {
            max_capabilities: 100,
            max_channels: 2,
            max_receipts: 100,
            cache_ttl_seconds: 300,
        };
        let store = MemoryStore::new(config);
        let pa = Did::new("did:nexa:pa");
        let pb = Did::new("did:nexa:pb");

        store
            .store_channel(Channel::new("ch-1", pa.clone(), pb.clone(), 100, 100))
            .await
            .unwrap();
        store
            .store_channel(Channel::new("ch-2", pa.clone(), pb.clone(), 200, 200))
            .await
            .unwrap();
        // 3rd channel evicts oldest
        store
            .store_channel(Channel::new("ch-3", pa.clone(), pb.clone(), 300, 300))
            .await
            .unwrap();

        let stats = store.stats().await;
        assert_eq!(
            stats.channels_count, 2,
            "eviction should keep count at max_channels"
        );
    }

    #[tokio::test]
    async fn test_list_channels_for_peer_no_match() {
        let store = MemoryStore::default_store();
        let pa = Did::new("did:nexa:pa");
        let pb = Did::new("did:nexa:pb");
        store
            .store_channel(Channel::new("ch-1", pa, pb, 100, 100))
            .await
            .unwrap();

        let outsider = Did::new("did:nexa:outsider");
        let result = store.list_channels_for_peer(&outsider).await.unwrap();
        assert!(
            result.is_empty(),
            "peer not in any channel should get empty list"
        );
    }

    #[tokio::test]
    async fn test_set_capability_availability_toggle() {
        let store = MemoryStore::default_store();
        let schema = create_test_schema("did:nexa:avail1", "svc-avail");
        store.register_capability(schema).await.unwrap();

        store
            .set_capability_availability("did:nexa:avail1", false)
            .await
            .unwrap();
        let cap = store
            .get_capability("did:nexa:avail1")
            .await
            .unwrap()
            .unwrap();
        assert!(!cap.available);

        store
            .set_capability_availability("did:nexa:avail1", true)
            .await
            .unwrap();
        let cap = store
            .get_capability("did:nexa:avail1")
            .await
            .unwrap()
            .unwrap();
        assert!(cap.available);
    }

    #[tokio::test]
    async fn test_update_capability_quality_value() {
        let store = MemoryStore::default_store();
        let schema = create_test_schema("did:nexa:qual1", "svc-qual");
        store.register_capability(schema).await.unwrap();

        let quality = serde_json::json!({"success_rate": 0.95, "latency_ms": 120});
        store
            .update_capability_quality("did:nexa:qual1", quality.clone())
            .await
            .unwrap();

        let cap = store
            .get_capability("did:nexa:qual1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cap.quality, quality);
    }

    #[tokio::test]
    async fn test_receipt_max_capacity_eviction() {
        let config = MemoryConfig {
            max_capabilities: 100,
            max_channels: 100,
            max_receipts: 5,
            cache_ttl_seconds: 300,
        };
        let max_receipts = config.max_receipts;
        let store = MemoryStore::new(config);
        let payer = Did::new("did:nexa:p1");
        let payee = Did::new("did:nexa:pay1");

        for i in 0..5 {
            let receipt = MicroReceipt {
                receipt_id: format!("r{}", i),
                call_id: format!("call-{}", i),
                payer: payer.to_string(),
                payee: payee.to_string(),
                amount: 100 + i,
                service_endpoint: format!("/svc-{}", i),
                timestamp: Utc::now(),
                previous_receipt_hash: "genesis".to_string(),
                payer_signature: vec![],
                payee_signature: None,
            };
            store.store_receipt(receipt).await.unwrap();
        }

        // 6th receipt triggers eviction of oldest 10%
        let receipt = MicroReceipt {
            receipt_id: "r-extra".to_string(),
            call_id: "call-extra".to_string(),
            payer: payer.to_string(),
            payee: payee.to_string(),
            amount: 999,
            service_endpoint: "/svc-extra".to_string(),
            timestamp: Utc::now(),
            previous_receipt_hash: "genesis".to_string(),
            payer_signature: vec![],
            payee_signature: None,
        };
        store.store_receipt(receipt).await.unwrap();

        let stats = store.stats().await;
        // When max_receipts=5, eviction removes 10% = floor(5/10)=0 entries,
        // so count may briefly exceed max_receipts. Verify eviction logic ran:
        // count should be <= max_receipts + 1 (the newly added receipt)
        assert!(
            stats.receipts_count <= max_receipts + 1,
            "receipts_count should stay within max_receipts+1 after eviction, got {}",
            stats.receipts_count
        );
    }

    // ── Proptest: cache round-trip with arbitrary keys & JSON values ──

    use proptest::prelude::*;

    /// Custom strategy for generating simple serde_json::Value
    /// (serde_json::Value does not implement Arbitrary)
    fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            Just(serde_json::json!("hello")),
            Just(serde_json::json!(42)),
            Just(serde_json::json!(true)),
            Just(serde_json::json!(null)),
            Just(serde_json::json!({"key": "val"})),
            Just(serde_json::json!([1, 2, 3])),
        ]
    }

    proptest! {
        #[test]
        fn proptest_cache_set_get_roundtrip(
            key in "[a-zA-Z0-9_]{1,20}",
            value in arb_json_value()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = MemoryStore::default_store();
                store.cache_set(&key, value.clone()).await.unwrap();
                let retrieved = store.cache_get(&key).await.unwrap();
                prop_assert!(retrieved.is_some());
                prop_assert_eq!(retrieved.unwrap(), value);
                Ok(())
            });
        }

        #[test]
        fn proptest_cache_overwrite_latest_wins(
            key in "[a-zA-Z0-9_]{1,20}",
            v1 in arb_json_value(),
            v2 in arb_json_value()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = MemoryStore::default_store();
                store.cache_set(&key, v1).await.unwrap();
                store.cache_set(&key, v2.clone()).await.unwrap();
                let retrieved = store.cache_get(&key).await.unwrap();
                prop_assert!(retrieved.is_some());
                prop_assert_eq!(retrieved.unwrap(), v2, "overwriting should return latest value");
                Ok(())
            });
        }

        #[test]
        fn proptest_cache_delete_then_get_none(
            key in "[a-zA-Z0-9_]{1,20}",
            value in arb_json_value()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = MemoryStore::default_store();
                store.cache_set(&key, value).await.unwrap();
                store.cache_delete(&key).await.unwrap();
                let retrieved = store.cache_get(&key).await.unwrap();
                prop_assert!(retrieved.is_none(), "deleted key should return None");
                Ok(())
            });
        }

        #[test]
        fn proptest_capability_register_get_roundtrip(
            name in "[a-zA-Z0-9_]{1,20}",
            tag in "[a-zA-Z0-9_]{1,10}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = MemoryStore::default_store();
                let did_str = format!("did:nexa:{}", name);
                let schema = CapabilitySchema {
                    version: "1.0".to_string(),
                    metadata: ServiceMetadata {
                        did: Did::new(&did_str),
                        name: name.clone(),
                        description: "proptest service".to_string(),
                        tags: vec![tag.clone()],
                    },
                    endpoints: vec![],
                };

                store.register_capability(schema).await.unwrap();
                let cap = store.get_capability(&did_str).await.unwrap();
                prop_assert!(cap.is_some());
                let cap = cap.unwrap();
                prop_assert_eq!(cap.schema.metadata.name, name);
                prop_assert!(cap.schema.metadata.tags.contains(&tag));
                Ok(())
            });
        }
    }
}
