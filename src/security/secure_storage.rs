//! Secure Key Storage
//!
//! Encrypted storage for sensitive key material using AES-256-GCM.
//! All operations produce audit events when an AuditLogger is configured.
//!
//! # Performance
//!
//! Uses `DashMap` instead of `RwLock<HashMap>` for concurrent access.
//! This eliminates the async write-lock bottleneck — each key entry is
//! independently locked (sharded), allowing concurrent operations on
//! different keys without blocking. All methods are now synchronous,
//! removing the overhead of async runtime scheduling.

use crate::security::{AuditEvent, AuditLogger, SecurityError, SecurityResult};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use zeroize::Zeroize;

/// Key metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key identifier
    pub key_id: String,
    /// Key type (signing, encryption, etc.)
    pub key_type: String,
    /// Key version (incremented on rotation)
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

/// Encrypted key entry (stored internally)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedKeyEntry {
    /// Encrypted key data (base64-encoded AES-256-GCM ciphertext + auth tag)
    encrypted_data: String,
    /// Nonce used for encryption (base64-encoded 12-byte nonce)
    nonce: String,
    /// Key metadata
    metadata: KeyMetadata,
}

/// Secure key storage with AES-256-GCM encryption
///
/// Uses `DashMap` for sharded concurrent access, eliminating the global
/// write-lock bottleneck of `RwLock<HashMap>`. Each key entry is independently
/// locked, allowing concurrent operations on different keys.
///
/// All methods are synchronous — no async overhead for lock acquisition.
pub struct SecureKeyStorage {
    /// AES-256-GCM cipher instance (None when encryption is disabled)
    cipher: Option<Aes256Gcm>,
    /// Stored keys (key_id -> encrypted entry) — DashMap for concurrent access
    keys: DashMap<String, EncryptedKeyEntry>,
    /// Whether encryption is enabled
    encryption_enabled: bool,
    /// Optional audit logger for security event tracking
    audit_logger: Option<Arc<AuditLogger>>,
}

impl SecureKeyStorage {
    /// Create a new secure key storage with AES-256-GCM encryption
    ///
    /// When `encryption_key` is provided, all key data will be encrypted
    /// with AES-256-GCM before storage. When `None`, data is stored
    /// unencrypted (only for testing — never use in production).
    pub fn new(encryption_key: Option<[u8; 32]>) -> Self {
        let cipher = encryption_key.map(|key| {
            let key = Key::<Aes256Gcm>::from_slice(&key);
            Aes256Gcm::new(key)
        });
        let encryption_enabled = cipher.is_some();
        Self {
            cipher,
            keys: DashMap::new(),
            encryption_enabled,
            audit_logger: None,
        }
    }

    /// Create without encryption (for testing only)
    ///
    /// **WARNING**: Never use this in production. Data stored with `insecure()`
    /// is not encrypted and is vulnerable to extraction.
    pub fn insecure() -> Self {
        Self {
            cipher: None,
            keys: DashMap::new(),
            encryption_enabled: false,
            audit_logger: None,
        }
    }

    /// Set audit logger for security event tracking
    pub fn with_audit_logger(mut self, logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// Store a key with encryption
    ///
    /// The key data is encrypted with AES-256-GCM using a fresh random nonce.
    /// An audit event (`KeyGenerated`) is logged if an audit logger is set.
    pub fn store_key(
        &self,
        key_id: &str,
        key_type: &str,
        key_data: &[u8],
        description: Option<&str>,
    ) -> SecurityResult<()> {
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
            // Store unencrypted (for testing only)
            EncryptedKeyEntry {
                encrypted_data: base64_encode(key_data),
                nonce: String::new(),
                metadata,
            }
        };

        self.keys.insert(key_id.to_string(), entry);

        if let Some(logger) = &self.audit_logger {
            if let Err(e) = logger.log_key_generated(key_id, key_type) {
                tracing::warn!("Failed to log audit event: {}", e);
            }
        }

        Ok(())
    }

    /// Retrieve a key with decryption
    ///
    /// The key data is decrypted from AES-256-GCM storage.
    /// An audit event (`KeyAccessed`) is logged if an audit logger is set.
    pub fn get_key(&self, key_id: &str) -> SecurityResult<Option<(Vec<u8>, KeyMetadata)>> {
        if let Some(mut entry) = self.keys.get_mut(key_id) {
            entry.metadata.last_accessed = Utc::now();

            let key_data = if self.encryption_enabled {
                self.decrypt_key_data(&entry)?
            } else {
                base64_decode(&entry.encrypted_data)?
            };

            if let Some(logger) = &self.audit_logger {
                if let Err(e) = logger.log(AuditEvent::KeyAccessed {
                    key_id: key_id.to_string(),
                    accessor: "internal".to_string(),
                    operation: "retrieve".to_string(),
                    timestamp: Utc::now(),
                }) {
                    tracing::warn!("Failed to log audit event: {}", e);
                }
            }

            Ok(Some((key_data, entry.metadata.clone())))
        } else {
            Ok(None)
        }
    }

    /// Check if a key exists
    pub fn has_key(&self, key_id: &str) -> bool {
        self.keys.contains_key(key_id)
    }

    /// Delete a key (zeroizes encrypted material)
    pub fn delete_key(&self, key_id: &str) -> SecurityResult<()> {
        if let Some((_, mut entry)) = self.keys.remove(key_id) {
            // Zeroize the encrypted data and nonce to prevent memory forensic extraction
            entry.encrypted_data.zeroize();
            entry.nonce.zeroize();
        }

        Ok(())
    }

    /// List all key metadata (without exposing key data)
    pub fn list_keys(&self) -> SecurityResult<Vec<KeyMetadata>> {
        Ok(self.keys.iter().map(|e| e.metadata.clone()).collect())
    }

    /// Update key metadata
    pub fn update_metadata(
        &self,
        key_id: &str,
        description: Option<&str>,
        is_active: Option<bool>,
    ) -> SecurityResult<()> {
        if let Some(mut entry) = self.keys.get_mut(key_id) {
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

    /// Rotate a key (re-encrypt with new data and incremented version)
    ///
    /// The old encrypted entry is replaced with a freshly encrypted one
    /// using a new random nonce. An audit event (`KeyRotated`) is logged.
    pub fn rotate_key(&self, key_id: &str, new_key_data: &[u8]) -> SecurityResult<u32> {
        if let Some(mut entry) = self.keys.get_mut(key_id) {
            let old_version = entry.metadata.version;
            let new_version = old_version + 1;
            entry.metadata.version = new_version;
            entry.metadata.last_accessed = Utc::now();

            if self.encryption_enabled {
                // Re-encrypt with fresh nonce for forward secrecy
                let new_entry = self.encrypt_key_data(new_key_data, entry.metadata.clone())?;
                *entry = new_entry;
            } else {
                entry.encrypted_data = base64_encode(new_key_data);
            }

            if let Some(logger) = &self.audit_logger {
                if let Err(e) = logger.log_key_rotated(key_id, old_version, new_version) {
                    tracing::warn!("Failed to log audit event: {}", e);
                }
            }

            Ok(new_version)
        } else {
            Err(SecurityError::KeyNotFound(key_id.to_string()))
        }
    }

    /// Get storage statistics
    pub fn stats(&self) -> StorageStats {
        let total = self.keys.len();
        let active = self.keys.iter().filter(|e| e.metadata.is_active).count();
        let by_type = self.keys.iter().fold(HashMap::new(), |mut acc, e| {
            *acc.entry(e.metadata.key_type.clone()).or_insert(0) += 1;
            acc
        });

        StorageStats {
            total_keys: total,
            active_keys: active,
            keys_by_type: by_type,
            encryption_enabled: self.encryption_enabled,
            encryption_algorithm: if self.encryption_enabled {
                "AES-256-GCM"
            } else {
                "none"
            },
        }
    }

    /// Encrypt key data with AES-256-GCM
    ///
    /// AES-256-GCM provides both confidentiality (encryption) and
    /// authenticity (authentication tag), making it an AEAD cipher.
    /// Each call generates a fresh random 12-byte nonce to ensure
    /// semantic security even for identical plaintexts.
    fn encrypt_key_data(
        &self,
        key_data: &[u8],
        metadata: KeyMetadata,
    ) -> SecurityResult<EncryptedKeyEntry> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or_else(|| SecurityError::EncryptionError("No encryption key set".to_string()))?;

        // Generate a random 12-byte nonce for AES-256-GCM
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt with AES-256-GCM (provides confidentiality + authenticity)
        let encrypted = cipher.encrypt(&nonce, key_data).map_err(|e| {
            SecurityError::EncryptionError(format!("AES-GCM encryption failed: {}", e))
        })?;

        Ok(EncryptedKeyEntry {
            encrypted_data: base64_encode(&encrypted),
            nonce: base64_encode(nonce.as_slice()),
            metadata,
        })
    }

    /// Decrypt key data with AES-256-GCM
    ///
    /// Decryption verifies the authentication tag before releasing
    /// plaintext. If the tag verification fails (e.g., data was
    /// tampered or wrong key was used), decryption returns an error.
    fn decrypt_key_data(&self, entry: &EncryptedKeyEntry) -> SecurityResult<Vec<u8>> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or_else(|| SecurityError::EncryptionError("No encryption key set".to_string()))?;

        let encrypted = base64_decode(&entry.encrypted_data)?;
        let nonce_bytes = base64_decode(&entry.nonce)?;

        if nonce_bytes.len() != 12 {
            return Err(SecurityError::EncryptionError(
                "Invalid nonce length: expected 12 bytes for AES-256-GCM".to_string(),
            ));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt with AES-256-GCM (verifies authenticity before decryption)
        cipher.decrypt(nonce, encrypted.as_slice()).map_err(|e| {
            SecurityError::EncryptionError(format!("AES-GCM decryption failed: {}", e))
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of stored keys
    pub total_keys: usize,
    /// Number of active keys
    pub active_keys: usize,
    /// Key count by type
    pub keys_by_type: HashMap<String, usize>,
    /// Whether encryption is enabled
    pub encryption_enabled: bool,
    /// Encryption algorithm in use ("AES-256-GCM" or "none")
    pub encryption_algorithm: &'static str,
}

/// Base64 encode helper
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Base64 decode helper
fn base64_decode(s: &str) -> SecurityResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| SecurityError::EncryptionError(format!("Base64 decode error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::audit::MemoryAuditSink;

    #[test]
    fn test_store_and_retrieve_key() {
        let storage = SecureKeyStorage::insecure();
        let key_data = b"test-key-data-123";

        storage
            .store_key("key-1", "signing", key_data, Some("Test key"))
            .unwrap();

        let result = storage.get_key("key-1").unwrap();
        assert!(result.is_some());

        let (retrieved_data, metadata) = result.unwrap();
        assert_eq!(retrieved_data, key_data.to_vec());
        assert_eq!(metadata.key_type, "signing");
        assert_eq!(metadata.description, Some("Test key".to_string()));
    }

    #[test]
    fn test_key_not_found() {
        let storage = SecureKeyStorage::insecure();

        let result = storage.get_key("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_key() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data", None)
            .unwrap();
        assert!(storage.has_key("key-1"));

        storage.delete_key("key-1").unwrap();
        assert!(!storage.has_key("key-1"));
    }

    #[test]
    fn test_rotate_key() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"old-data", None)
            .unwrap();

        let new_version = storage.rotate_key("key-1", b"new-data").unwrap();
        assert_eq!(new_version, 2);

        let (data, metadata) = storage.get_key("key-1").unwrap().unwrap();
        assert_eq!(data, b"new-data".to_vec());
        assert_eq!(metadata.version, 2);
    }

    #[test]
    fn test_list_keys() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data1", None)
            .unwrap();
        storage
            .store_key("key-2", "encryption", b"data2", None)
            .unwrap();

        let keys = storage.list_keys().unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_encrypted_storage_aes_gcm() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        let key_data = b"secret-key-data";
        storage
            .store_key("key-1", "signing", key_data, None)
            .unwrap();

        let (retrieved, _) = storage.get_key("key-1").unwrap().unwrap();
        assert_eq!(retrieved, key_data.to_vec());

        // Verify encryption is AES-256-GCM
        let stats = storage.stats();
        assert_eq!(stats.encryption_algorithm, "AES-256-GCM");
        assert!(stats.encryption_enabled);
    }

    #[test]
    fn test_aes_gcm_wrong_key_fails() {
        // Store with one key, attempt to decrypt with another — must fail
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        storage
            .store_key("key-1", "signing", b"secret-data", None)
            .unwrap();

        // Create a different storage with wrong key
        let wrong_key = [99u8; 32];
        let wrong_storage = SecureKeyStorage::new(Some(wrong_key));

        // Copy the encrypted entry to the wrong storage
        {
            let entry = storage.keys.get("key-1").unwrap().clone();
            wrong_storage.keys.insert("key-1".to_string(), entry);
        }

        // Decryption with wrong key must fail (auth tag verification)
        let result = wrong_storage.get_key("key-1");
        assert!(
            result.is_err(),
            "AES-GCM should reject decryption with wrong key"
        );
    }

    #[test]
    fn test_aes_gcm_tampered_data_fails() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        storage
            .store_key("key-1", "signing", b"secret-data", None)
            .unwrap();

        // Tamper with the encrypted data
        {
            let mut entry = storage.keys.get_mut("key-1").unwrap();
            // Modify ciphertext — AES-GCM auth tag will reject
            let mut encrypted = base64_decode(&entry.encrypted_data).unwrap();
            if !encrypted.is_empty() {
                encrypted[0] ^= 0xFF; // Flip first byte
            }
            entry.encrypted_data = base64_encode(&encrypted);
        }

        // Decryption of tampered data must fail
        let result = storage.get_key("key-1");
        assert!(result.is_err(), "AES-GCM should reject tampered ciphertext");
    }

    #[test]
    fn test_storage_stats() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data", None)
            .unwrap();
        storage
            .store_key("key-2", "signing", b"data", None)
            .unwrap();
        storage
            .store_key("key-3", "encryption", b"data", None)
            .unwrap();

        let stats = storage.stats();
        assert_eq!(stats.total_keys, 3);
        assert_eq!(stats.active_keys, 3);
        assert_eq!(*stats.keys_by_type.get("signing").unwrap_or(&0), 2);
        assert_eq!(*stats.keys_by_type.get("encryption").unwrap_or(&0), 1);
        assert_eq!(stats.encryption_algorithm, "none");
    }

    #[tokio::test]
    async fn test_audit_logger_integration() {
        let sink = Arc::new(MemoryAuditSink::new(100));
        let logger = Arc::new(AuditLogger::new(sink.clone()));

        let storage = SecureKeyStorage::insecure().with_audit_logger(logger);

        storage
            .store_key("key-1", "signing", b"data", None)
            .unwrap();

        // Verify KeyGenerated audit event was logged
        let events = sink.get_events_by_type("key_generated").await;
        assert_eq!(events.len(), 1);

        // Verify KeyAccessed audit event on retrieval
        storage.get_key("key-1").unwrap();
        let access_events = sink.get_events_by_type("key_accessed").await;
        assert_eq!(access_events.len(), 1);

        // Verify KeyRotated audit event on rotation
        storage.rotate_key("key-1", b"new-data").unwrap();
        let rotate_events = sink.get_events_by_type("key_rotated").await;
        assert_eq!(rotate_events.len(), 1);
    }

    #[test]
    fn test_aes_gcm_large_key_data() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        // Test with larger key data (1KB)
        let large_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        storage
            .store_key("key-large", "encryption", &large_data, None)
            .unwrap();

        let (retrieved, _) = storage.get_key("key-large").unwrap().unwrap();
        assert_eq!(retrieved, large_data);
    }

    #[test]
    fn test_aes_gcm_rotation_re_encrypts() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        storage
            .store_key("key-1", "signing", b"old-data", None)
            .unwrap();

        // Get the nonce from the original entry
        let original_nonce = {
            let entry = storage.keys.get("key-1").unwrap();
            entry.nonce.clone()
        };

        // Rotate — should generate a fresh nonce (forward secrecy)
        storage.rotate_key("key-1", b"new-data").unwrap();

        let new_nonce = {
            let entry = storage.keys.get("key-1").unwrap();
            entry.nonce.clone()
        };

        // Nonces must differ (fresh nonce on re-encryption)
        assert_ne!(original_nonce, new_nonce, "Rotation must use a fresh nonce");
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_empty_key_data_store_and_retrieve() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        // Store empty key data
        storage
            .store_key("key-empty", "signing", b"", None)
            .unwrap();

        let (retrieved, _) = storage.get_key("key-empty").unwrap().unwrap();
        assert_eq!(retrieved, Vec::<u8>::new());
    }

    #[test]
    fn test_delete_nonexistent_key_ok() {
        let storage = SecureKeyStorage::insecure();

        // Deleting a nonexistent key should not error (just does nothing)
        storage.delete_key("nonexistent").unwrap();
        assert!(!storage.has_key("nonexistent"));
    }

    #[test]
    fn test_update_metadata_nonexistent_key() {
        let storage = SecureKeyStorage::insecure();

        let result = storage.update_metadata("nonexistent", Some("desc"), Some(false));
        assert!(result.is_err());
    }

    #[test]
    fn test_update_metadata_set_active() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data", None)
            .unwrap();

        // Deactivate the key
        storage.update_metadata("key-1", None, Some(false)).unwrap();

        let (_, metadata) = storage.get_key("key-1").unwrap().unwrap();
        assert!(!metadata.is_active);

        // Stats should reflect inactive key
        let stats = storage.stats();
        assert_eq!(stats.active_keys, 0);
    }

    #[test]
    fn test_update_metadata_set_description() {
        let storage = SecureKeyStorage::insecure();

        storage
            .store_key("key-1", "signing", b"data", None)
            .unwrap();

        storage
            .update_metadata("key-1", Some("Updated description"), None)
            .unwrap();

        let (_, metadata) = storage.get_key("key-1").unwrap().unwrap();
        assert_eq!(
            metadata.description,
            Some("Updated description".to_string())
        );
    }

    #[test]
    fn test_rotate_nonexistent_key() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        let result = storage.rotate_key("nonexistent", b"new-data");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_key_nonexistent() {
        let storage = SecureKeyStorage::insecure();
        assert!(!storage.has_key("nonexistent"));
    }

    #[test]
    fn test_list_keys_empty() {
        let storage = SecureKeyStorage::insecure();
        let keys = storage.list_keys().unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_stats_empty_storage() {
        let encryption_key = [42u8; 32];
        let storage = SecureKeyStorage::new(Some(encryption_key));

        let stats = storage.stats();
        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.active_keys, 0);
        assert!(stats.keys_by_type.is_empty());
        assert!(stats.encryption_enabled);
    }

    #[test]
    fn test_insecure_stats() {
        let storage = SecureKeyStorage::insecure();
        let stats = storage.stats();
        assert!(!stats.encryption_enabled);
        assert_eq!(stats.encryption_algorithm, "none");
    }

    #[test]
    fn test_multiple_rotation_versions() {
        let storage = SecureKeyStorage::insecure();

        storage.store_key("key-1", "signing", b"v1", None).unwrap();

        let v2 = storage.rotate_key("key-1", b"v2").unwrap();
        assert_eq!(v2, 2);

        let v3 = storage.rotate_key("key-1", b"v3").unwrap();
        assert_eq!(v3, 3);

        let (data, metadata) = storage.get_key("key-1").unwrap().unwrap();
        assert_eq!(data, b"v3".to_vec());
        assert_eq!(metadata.version, 3);
    }

    #[test]
    fn test_store_overwrite_existing_key() {
        let storage = SecureKeyStorage::insecure();

        storage.store_key("key-1", "signing", b"old", None).unwrap();
        storage
            .store_key("key-1", "encryption", b"new", None)
            .unwrap();

        let (data, metadata) = storage.get_key("key-1").unwrap().unwrap();
        assert_eq!(data, b"new".to_vec());
        assert_eq!(metadata.key_type, "encryption");
    }

    #[test]
    fn test_dashmap_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let storage = Arc::new(SecureKeyStorage::insecure());

        let mut handles = vec![];

        // 10 threads each storing and retrieving 100 keys
        for i in 0..10 {
            let storage_clone = storage.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let key_id = format!("key-{}-{}", i, j);
                    storage_clone
                        .store_key(&key_id, "signing", b"test-data", None)
                        .unwrap();
                    let result = storage_clone.get_key(&key_id).unwrap();
                    assert!(result.is_some());
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all keys are stored
        let stats = storage.stats();
        assert_eq!(stats.total_keys, 1000);
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// AES-256-GCM encrypt → decrypt round-trip with arbitrary key data
        #[test]
        fn proptest_aes_gcm_roundtrip(
            key_data in prop::collection::vec(any::<u8>(), 0..1024),
        ) {
            let encryption_key: [u8; 32] = [42u8; 32];
            let storage = SecureKeyStorage::new(Some(encryption_key));

            storage
                .store_key("key-proptest", "signing", &key_data, None)
                .unwrap();

            let (retrieved, _) = storage.get_key("key-proptest").unwrap().unwrap();
            assert_eq!(retrieved, key_data);
        }
    }
}
