//! Secure Key Storage
//!
//! Encrypted storage for sensitive key material.

use crate::security::{SecurityError, SecurityResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroize::Zeroize;

/// Key metadata for storage
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
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Whether the key is active
    pub is_active: bool,
    /// Optional description
    pub description: Option<String>,
}

/// Encrypted key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedKeyEntry {
    /// Encrypted key data (base64)
    encrypted_data: String,
    /// Nonce/IV used for encryption (base64)
    nonce: String,
    /// Key metadata
    metadata: KeyMetadata,
}

/// Secure key storage
pub struct SecureKeyStorage {
    /// Encryption key for storage
    encryption_key: Option<[u8; 32]>,
    /// Stored keys (key_id -> encrypted entry)
    keys: Arc<RwLock<HashMap<String, EncryptedKeyEntry>>>,
    /// Whether encryption is enabled
    encryption_enabled: bool,
}

impl SecureKeyStorage {
    /// Create a new secure key storage
    pub fn new(encryption_key: Option<[u8; 32]>) -> Self {
        Self {
            encryption_key,
            keys: Arc::new(RwLock::new(HashMap::new())),
            encryption_enabled: true,
        }
    }

    /// Create without encryption (for testing only)
    pub fn insecure() -> Self {
        Self {
            encryption_key: None,
            keys: Arc::new(RwLock::new(HashMap::new())),
            encryption_enabled: false,
        }
    }

    /// Store a key
    pub async fn store_key(
        &self,
        key_id: &str,
        key_type: &str,
        key_data: &[u8],
        description: Option<&str>,
    ) -> SecurityResult<()> {
        let mut keys = self.keys.write().await;

        let metadata = KeyMetadata {
            key_id: key_id.to_string(),
            key_type: key_type.to_string(),
            version: 1,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            is_active: true,
            description: description.map(|s| s.to_string()),
        };

        let entry = if self.encryption_enabled {
            self.encrypt_key_data(key_data, metadata)?
        } else {
            // Store unencrypted (for testing)
            EncryptedKeyEntry {
                encrypted_data: base64_encode(key_data),
                nonce: String::new(),
                metadata,
            }
        };

        keys.insert(key_id.to_string(), entry);
        Ok(())
    }

    /// Retrieve a key
    pub async fn get_key(&self, key_id: &str) -> SecurityResult<Option<(Vec<u8>, KeyMetadata)>> {
        let mut keys = self.keys.write().await;

        if let Some(entry) = keys.get_mut(key_id) {
            entry.metadata.last_accessed = Utc::now();

            let key_data = if self.encryption_enabled {
                self.decrypt_key_data(entry)?
            } else {
                base64_decode(&entry.encrypted_data)?
            };

            Ok(Some((key_data, entry.metadata.clone())))
        } else {
            Ok(None)
        }
    }

    /// Check if a key exists
    pub async fn has_key(&self, key_id: &str) -> bool {
        let keys = self.keys.read().await;
        keys.contains_key(key_id)
    }

    /// Delete a key
    pub async fn delete_key(&self, key_id: &str) -> SecurityResult<()> {
        let mut keys = self.keys.write().await;

        if let Some(mut entry) = keys.remove(key_id) {
            // Zeroize the encrypted data
            entry.encrypted_data.zeroize();
            entry.nonce.zeroize();
        }

        Ok(())
    }

    /// List all key metadata (without key data)
    pub async fn list_keys(&self) -> SecurityResult<Vec<KeyMetadata>> {
        let keys = self.keys.read().await;
        Ok(keys.values().map(|e| e.metadata.clone()).collect())
    }

    /// Update key metadata
    pub async fn update_metadata(
        &self,
        key_id: &str,
        description: Option<&str>,
        is_active: Option<bool>,
    ) -> SecurityResult<()> {
        let mut keys = self.keys.write().await;

        if let Some(entry) = keys.get_mut(key_id) {
            if let Some(desc) = description {
                entry.metadata.description = Some(desc.to_string());
            }
            if let Some(active) = is_active {
                entry.metadata.is_active = active;
            }
            Ok(())
        } else {
            Err(SecurityError::KeyNotFound(key_id.to_string()))
        }
    }

    /// Rotate a key (store new version)
    pub async fn rotate_key(&self, key_id: &str, new_key_data: &[u8]) -> SecurityResult<u32> {
        let mut keys = self.keys.write().await;

        if let Some(entry) = keys.get_mut(key_id) {
            let new_version = entry.metadata.version + 1;
            entry.metadata.version = new_version;
            entry.metadata.last_accessed = Utc::now();

            if self.encryption_enabled {
                let new_entry = self.encrypt_key_data(new_key_data, entry.metadata.clone())?;
                *entry = new_entry;
            } else {
                entry.encrypted_data = base64_encode(new_key_data);
            }

            Ok(new_version)
        } else {
            Err(SecurityError::KeyNotFound(key_id.to_string()))
        }
    }

    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        let keys = self.keys.read().await;

        let total = keys.len();
        let active = keys.values().filter(|e| e.metadata.is_active).count();
        let by_type = keys.values().fold(HashMap::new(), |mut acc, e| {
            *acc.entry(e.metadata.key_type.clone()).or_insert(0) += 1;
            acc
        });

        StorageStats {
            total_keys: total,
            active_keys: active,
            keys_by_type: by_type,
            encryption_enabled: self.encryption_enabled,
        }
    }

    /// Encrypt key data
    fn encrypt_key_data(
        &self,
        key_data: &[u8],
        metadata: KeyMetadata,
    ) -> SecurityResult<EncryptedKeyEntry> {
        // Simple XOR encryption for demonstration
        // In production, use proper AEAD encryption like AES-GCM or ChaCha20-Poly1305
        let key = self
            .encryption_key
            .ok_or_else(|| SecurityError::EncryptionError("No encryption key set".to_string()))?;

        let nonce = generate_nonce();
        let encrypted = xor_encrypt(key_data, &key, &nonce);

        Ok(EncryptedKeyEntry {
            encrypted_data: base64_encode(&encrypted),
            nonce: base64_encode(&nonce),
            metadata,
        })
    }

    /// Decrypt key data
    fn decrypt_key_data(&self, entry: &EncryptedKeyEntry) -> SecurityResult<Vec<u8>> {
        let key = self
            .encryption_key
            .ok_or_else(|| SecurityError::EncryptionError("No encryption key set".to_string()))?;

        let encrypted = base64_decode(&entry.encrypted_data)?;
        let nonce = base64_decode(&entry.nonce)?;

        Ok(xor_decrypt(&encrypted, &key, &nonce))
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub keys_by_type: HashMap<String, usize>,
    pub encryption_enabled: bool,
}

/// Generate a random nonce
fn generate_nonce() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..12).map(|_| rng.gen::<u8>()).collect()
}

/// Simple XOR encryption (for demonstration - use proper encryption in production)
fn xor_encrypt(data: &[u8], key: &[u8; 32], nonce: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| {
            let key_byte = key[i % 32];
            let nonce_byte = nonce.get(i % nonce.len()).copied().unwrap_or(0);
            byte ^ key_byte ^ nonce_byte
        })
        .collect()
}

/// XOR decryption (same as encryption for XOR)
fn xor_decrypt(data: &[u8], key: &[u8; 32], nonce: &[u8]) -> Vec<u8> {
    xor_encrypt(data, key, nonce)
}

/// Base64 encode
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Base64 decode
fn base64_decode(s: &str) -> SecurityResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| SecurityError::EncryptionError(format!("Base64 decode error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve_key() {
        let storage = SecureKeyStorage::insecure();
        let key_data = b"test-key-data-123";

        storage
            .store_key("key-1", "signing", key_data, Some("Test key"))
            .await
            .unwrap();

        let result = storage.get_key("key-1").await.unwrap();
        assert!(result.is_some());

        let (retrieved_data, metadata) = result.unwrap();
        assert_eq!(retrieved_data, key_data.to_vec());
        assert_eq!(metadata.key_type, "signing");
        assert_eq!(metadata.description, Some("Test key".to_string()));
    }

    #[tokio::test]
    async fn test_key_not_found() {
        let storage = SecureKeyStorage::insecure();

        let result = storage.get_key("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_key() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data", None)
            .await
            .unwrap();
        assert!(storage.has_key("key-1").await);

        storage.delete_key("key-1").await.unwrap();
        assert!(!storage.has_key("key-1").await);
    }

    #[tokio::test]
    async fn test_rotate_key() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"old-data", None)
            .await
            .unwrap();

        let new_version = storage.rotate_key("key-1", b"new-data").await.unwrap();
        assert_eq!(new_version, 2);

        let (data, metadata) = storage.get_key("key-1").await.unwrap().unwrap();
        assert_eq!(data, b"new-data".to_vec());
        assert_eq!(metadata.version, 2);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data1", None)
            .await
            .unwrap();
        storage
            .store_key("key-2", "encryption", b"data2", None)
            .await
            .unwrap();

        let keys = storage.list_keys().await.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[tokio::test]
    async fn test_encrypted_storage() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        let key_data = b"secret-key-data";
        storage
            .store_key("key-1", "signing", key_data, None)
            .await
            .unwrap();

        let (retrieved, _) = storage.get_key("key-1").await.unwrap().unwrap();
        assert_eq!(retrieved, key_data.to_vec());
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data", None)
            .await
            .unwrap();
        storage
            .store_key("key-2", "signing", b"data", None)
            .await
            .unwrap();
        storage
            .store_key("key-3", "encryption", b"data", None)
            .await
            .unwrap();

        let stats = storage.stats().await;
        assert_eq!(stats.total_keys, 3);
        assert_eq!(stats.active_keys, 3);
        assert_eq!(*stats.keys_by_type.get("signing").unwrap_or(&0), 2);
        assert_eq!(*stats.keys_by_type.get("encryption").unwrap_or(&0), 1);
    }
}
