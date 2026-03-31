//! Key Rotation Management
//!
//! Automatic key rotation for enhanced security.

use crate::error::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Key rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    /// Rotation interval in days
    pub rotation_interval_days: u32,
    /// Warning period before expiration (days)
    pub warning_period_days: u32,
    /// Maximum key age before forced rotation (days)
    pub max_key_age_days: u32,
    /// Enable automatic rotation
    pub auto_rotate: bool,
    /// Minimum key uses before rotation allowed
    pub min_uses_before_rotation: u64,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        Self {
            rotation_interval_days: 90,
            warning_period_days: 14,
            max_key_age_days: 365,
            auto_rotate: true,
            min_uses_before_rotation: 100,
        }
    }
}

/// Key metadata for rotation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key identifier
    pub key_id: String,
    /// Key type (signing, encryption, etc.)
    pub key_type: String,
    /// Key version
    pub version: u32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last rotation timestamp
    pub last_rotated_at: DateTime<Utc>,
    /// Number of times the key has been used
    pub use_count: u64,
    /// Whether the key is active
    pub is_active: bool,
    /// Next scheduled rotation
    pub next_rotation: DateTime<Utc>,
}

impl KeyMetadata {
    /// Create new key metadata
    pub fn new(key_id: &str, key_type: &str, policy: &KeyRotationPolicy) -> Self {
        let now = Utc::now();
        Self {
            key_id: key_id.to_string(),
            key_type: key_type.to_string(),
            version: 1,
            created_at: now,
            last_rotated_at: now,
            use_count: 0,
            is_active: true,
            next_rotation: now + Duration::days(policy.rotation_interval_days as i64),
        }
    }
    
    /// Check if rotation is due
    pub fn is_rotation_due(&self, policy: &KeyRotationPolicy) -> bool {
        let now = Utc::now();
        
        // Check time-based rotation
        if now >= self.next_rotation {
            return true;
        }
        
        // Check max age
        let age = (now - self.created_at).num_days();
        if age >= policy.max_key_age_days as i64 {
            return true;
        }
        
        false
    }
    
    /// Check if key is in warning period
    pub fn is_in_warning_period(&self, policy: &KeyRotationPolicy) -> bool {
        let now = Utc::now();
        let warning_start = self.next_rotation - Duration::days(policy.warning_period_days as i64);
        now >= warning_start && now < self.next_rotation
    }
    
    /// Record a key use
    pub fn record_use(&mut self) {
        self.use_count += 1;
    }
    
    /// Mark as rotated
    pub fn mark_rotated(&mut self, policy: &KeyRotationPolicy) {
        let now = Utc::now();
        self.version += 1;
        self.last_rotated_at = now;
        self.use_count = 0;
        self.next_rotation = now + Duration::days(policy.rotation_interval_days as i64);
    }
}

/// Key rotator
pub struct KeyRotator {
    policy: KeyRotationPolicy,
    keys: Arc<RwLock<HashMap<String, KeyMetadata>>>,
}

impl KeyRotator {
    /// Create a new key rotator
    pub fn new(policy: KeyRotationPolicy) -> Self {
        Self {
            policy,
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create with default policy
    pub fn default_rotator() -> Self {
        Self::new(KeyRotationPolicy::default())
    }
    
    /// Register a key for rotation tracking
    pub async fn register_key(&self, key_id: &str, key_type: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        let metadata = KeyMetadata::new(key_id, key_type, &self.policy);
        keys.insert(key_id.to_string(), metadata);
        Ok(())
    }
    
    /// Unregister a key
    pub async fn unregister_key(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        keys.remove(key_id);
        Ok(())
    }
    
    /// Record key usage
    pub async fn record_key_use(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        if let Some(metadata) = keys.get_mut(key_id) {
            metadata.record_use();
        }
        Ok(())
    }
    
    /// Check if a key needs rotation
    pub async fn needs_rotation(&self, key_id: &str) -> Result<bool> {
        let keys = self.keys.read().await;
        if let Some(metadata) = keys.get(key_id) {
            Ok(metadata.is_rotation_due(&self.policy))
        } else {
            Ok(false)
        }
    }
    
    /// Get keys that need rotation
    pub async fn get_keys_for_rotation(&self) -> Result<Vec<String>> {
        let keys = self.keys.read().await;
        Ok(keys.iter()
            .filter(|(_, m)| m.is_rotation_due(&self.policy))
            .map(|(k, _)| k.clone())
            .collect())
    }
    
    /// Get keys in warning period
    pub async fn get_keys_in_warning(&self) -> Result<Vec<(String, DateTime<Utc>)>> {
        let keys = self.keys.read().await;
        Ok(keys.iter()
            .filter(|(_, m)| m.is_in_warning_period(&self.policy))
            .map(|(k, m)| (k.clone(), m.next_rotation))
            .collect())
    }
    
    /// Mark a key as rotated
    pub async fn mark_rotated(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        if let Some(metadata) = keys.get_mut(key_id) {
            metadata.mark_rotated(&self.policy);
        }
        Ok(())
    }
    
    /// Get key metadata
    pub async fn get_metadata(&self, key_id: &str) -> Result<Option<KeyMetadata>> {
        let keys = self.keys.read().await;
        Ok(keys.get(key_id).cloned())
    }
    
    /// Get all key metadata
    pub async fn get_all_metadata(&self) -> Result<Vec<KeyMetadata>> {
        let keys = self.keys.read().await;
        Ok(keys.values().cloned().collect())
    }
    
    /// Get rotation statistics
    pub async fn stats(&self) -> RotationStats {
        let keys = self.keys.read().await;
        
        let total = keys.len();
        let active = keys.values().filter(|m| m.is_active).count();
        let pending_rotation = keys.values()
            .filter(|m| m.is_rotation_due(&self.policy))
            .count();
        let in_warning = keys.values()
            .filter(|m| m.is_in_warning_period(&self.policy))
            .count();
        
        RotationStats {
            total_keys: total,
            active_keys: active,
            pending_rotation,
            in_warning_period: in_warning,
        }
    }
}

/// Rotation statistics
#[derive(Debug, Clone)]
pub struct RotationStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub pending_rotation: usize,
    pub in_warning_period: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_registration() {
        let rotator = KeyRotator::default_rotator();
        
        rotator.register_key("key-1", "signing").await.unwrap();
        
        let metadata = rotator.get_metadata("key-1").await.unwrap();
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().key_type, "signing");
    }

    #[tokio::test]
    async fn test_key_rotation_check() {
        let mut policy = KeyRotationPolicy::default();
        policy.rotation_interval_days = 0; // Immediate rotation
        
        let rotator = KeyRotator::new(policy);
        rotator.register_key("key-1", "signing").await.unwrap();
        
        // Should need rotation immediately due to 0 day interval
        let needs_rotation = rotator.needs_rotation("key-1").await.unwrap();
        assert!(needs_rotation);
    }

    #[tokio::test]
    async fn test_key_usage_tracking() {
        let rotator = KeyRotator::default_rotator();
        rotator.register_key("key-1", "signing").await.unwrap();
        
        for _ in 0..10 {
            rotator.record_key_use("key-1").await.unwrap();
        }
        
        let metadata = rotator.get_metadata("key-1").await.unwrap().unwrap();
        assert_eq!(metadata.use_count, 10);
    }

    #[tokio::test]
    async fn test_rotation_stats() {
        let rotator = KeyRotator::default_rotator();
        
        rotator.register_key("key-1", "signing").await.unwrap();
        rotator.register_key("key-2", "encryption").await.unwrap();
        
        let stats = rotator.stats().await;
        assert_eq!(stats.total_keys, 2);
        assert_eq!(stats.active_keys, 2);
    }
}