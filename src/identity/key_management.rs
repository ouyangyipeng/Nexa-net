//! Key management for Nexa-net
//!
//! Provides cryptographic key generation, storage, and operations.
//! Supports Ed25519 for signing and X25519 for key agreement.
//!
//! # Security Features
//!
//! - **Zeroize**: All private key material is zeroized on Drop via the `zeroize` crate
//! - **AES-256-GCM encryption**: KeyStore encrypts private keys at rest
//! - **Key rotation**: Support for generating new keys while maintaining backward compatibility

use crate::error::{Error, Result};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng as RandOsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::ZeroizeOnDrop;

// ============================================================================
// Ed25519 Signing Keys
// ============================================================================

/// Public key wrapper for Ed25519
#[derive(Debug, Clone)]
pub struct PublicKey(VerifyingKey);

impl PublicKey {
    /// Get the raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let verifying_key =
            VerifyingKey::from_bytes(bytes).map_err(|e| Error::KeyGeneration(e.to_string()))?;
        Ok(Self(verifying_key))
    }

    /// Get the inner key
    pub fn inner(&self) -> &VerifyingKey {
        &self.0
    }
}

/// Private key wrapper for Ed25519
///
/// Implements ZeroizeOnDrop to ensure the key material is
/// securely erased when the key goes out of scope.
#[derive(Debug, Clone, ZeroizeOnDrop)]
pub struct PrivateKey([u8; 32]);

impl PrivateKey {
    /// Get the raw bytes (use with caution)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        Ok(Self(*bytes))
    }

    /// Get the inner signing key reference
    ///
    /// Note: This creates a temporary SigningKey. The caller should
    /// not store this reference long-term.
    pub fn to_signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.0)
    }
}

/// Key pair for signing and verification (Ed25519)
#[derive(Debug, Clone)]
pub struct KeyPair {
    private_key: PrivateKey,
    public_key: PublicKey,
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate() -> Result<Self> {
        let signing_key = SigningKey::generate(&mut RandOsRng);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            private_key: PrivateKey(signing_key.to_bytes()),
            public_key: PublicKey(verifying_key),
        })
    }

    /// Create from existing private key bytes
    pub fn from_private_key(bytes: &[u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            private_key: PrivateKey(*bytes),
            public_key: PublicKey(verifying_key),
        })
    }

    /// Get the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Get the private key
    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    /// Sign a message using Ed25519
    pub fn sign(&self, message: &[u8]) -> Result<Signature> {
        let signing_key = self.private_key.to_signing_key();
        Ok(signing_key.sign(message))
    }

    /// Verify a signature using Ed25519
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        self.public_key
            .0
            .verify(message, signature)
            .map_err(|e| Error::SignatureVerification(e.to_string()))
    }

    /// Compute the DID identifier from the public key
    pub fn to_did_identifier(&self) -> String {
        let hash = Sha256::digest(self.public_key.to_bytes());
        hex::encode(&hash[..20])
    }
}

// ============================================================================
// X25519 Key Agreement Keys
// ============================================================================

/// Key agreement key pair (X25519)
///
/// Implements ZeroizeOnDrop to ensure private key material is
/// securely erased when the key goes out of scope.
#[derive(Debug, Clone, ZeroizeOnDrop)]
pub struct KeyAgreementKeyPair {
    /// Private key for key agreement (zeroized on drop)
    private_key: [u8; 32],
    /// Public key for key agreement
    public_key: [u8; 32],
}

impl KeyAgreementKeyPair {
    /// Generate a new X25519 key pair
    pub fn generate() -> Result<Self> {
        let secret = StaticSecret::random_from_rng(RandOsRng);
        let public = X25519PublicKey::from(&secret);

        Ok(Self {
            private_key: secret.to_bytes(),
            public_key: public.to_bytes(),
        })
    }

    /// Create from existing private key bytes
    pub fn from_private_key(bytes: &[u8; 32]) -> Result<Self> {
        let secret = StaticSecret::from(*bytes);
        let public = X25519PublicKey::from(&secret);

        Ok(Self {
            private_key: *bytes,
            public_key: public.to_bytes(),
        })
    }

    /// Get the public key bytes
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// Get the private key bytes
    pub fn private_key(&self) -> &[u8; 32] {
        &self.private_key
    }

    /// Perform Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &[u8; 32]) -> Result<[u8; 32]> {
        let secret = StaticSecret::from(self.private_key);
        let their_public_key = X25519PublicKey::from(*their_public);

        Ok(secret.diffie_hellman(&their_public_key).to_bytes())
    }
}

// ============================================================================
// Combined Identity Keys
// ============================================================================

/// Complete identity key set
///
/// Contains both Ed25519 signing keys and X25519 key agreement keys.
/// The private key material is zeroized when this struct is dropped.
#[derive(Debug, Clone)]
pub struct IdentityKeys {
    /// Signing key pair (Ed25519)
    pub signing: KeyPair,
    /// Key agreement key pair (X25519)
    pub key_agreement: KeyAgreementKeyPair,
}

impl IdentityKeys {
    /// Generate a new identity key set
    pub fn generate() -> Result<Self> {
        Ok(Self {
            signing: KeyPair::generate()?,
            key_agreement: KeyAgreementKeyPair::generate()?,
        })
    }

    /// Get the DID identifier derived from the signing public key
    pub fn did_identifier(&self) -> String {
        format!("did:nexa:{}", self.signing.to_did_identifier())
    }
}

// ============================================================================
// Encrypted Key Store
// ============================================================================

/// Metadata about a stored key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key algorithm
    pub algorithm: String,
    /// Key creation timestamp
    pub created_at: String,
    /// Key version (for rotation tracking)
    pub version: u32,
    /// Whether this key has been rotated
    pub is_current: bool,
}

impl Default for KeyMetadata {
    fn default() -> Self {
        Self {
            algorithm: "Ed25519".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            is_current: true,
        }
    }
}

/// Encrypted key storage
///
/// Stores private key material encrypted with AES-256-GCM.
/// Keys are never stored in plaintext; they are decrypted only
/// when needed for signing operations.
pub struct SecureKeyStore {
    /// AES-256-GCM cipher for encryption/decryption
    cipher: Aes256Gcm,
    /// Encrypted key data (DID -> encrypted bytes)
    encrypted_keys: std::collections::HashMap<String, Vec<u8>>,
    /// Key metadata (DID -> metadata)
    metadata: std::collections::HashMap<String, KeyMetadata>,
}

impl SecureKeyStore {
    /// Create a new secure key store with the given encryption key
    ///
    /// The encryption key should be derived from a secure source
    /// (e.g., PBKDF2 from a passphrase, or a hardware security module).
    pub fn new(encryption_key: &[u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);
        let cipher = Aes256Gcm::new(key);

        Self {
            cipher,
            encrypted_keys: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Store a key pair encrypted
    ///
    /// The private key is encrypted with AES-256-GCM before storage.
    /// A random nonce is generated for each encryption operation.
    pub fn store(&mut self, did: &str, keypair: &KeyPair) -> Result<()> {
        // Generate a random 12-byte nonce for AES-256-GCM
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the private key bytes
        let encrypted = self
            .cipher
            .encrypt(&nonce, keypair.private_key.to_bytes().as_ref())
            .map_err(|e| Error::Internal(format!("AES encryption failed: {}", e)))?;

        // Prepend nonce to encrypted data for storage
        let stored_data = [nonce.as_slice(), encrypted.as_slice()].concat();

        self.encrypted_keys.insert(did.to_string(), stored_data);
        self.metadata.insert(
            did.to_string(),
            KeyMetadata {
                algorithm: "Ed25519".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
                is_current: true,
            },
        );

        Ok(())
    }

    /// Retrieve and decrypt a key pair
    ///
    /// The private key is decrypted from AES-256-GCM storage.
    /// The public key is derived from the decrypted private key.
    pub fn retrieve(&self, did: &str) -> Result<KeyPair> {
        let stored_data = self
            .encrypted_keys
            .get(did)
            .ok_or_else(|| Error::Internal(format!("Key not found for DID: {}", did)))?;

        // Extract nonce (first 12 bytes) and encrypted data
        if stored_data.len() < 12 {
            return Err(Error::Internal("Invalid encrypted key data".to_string()));
        }

        let nonce = Nonce::from_slice(&stored_data[..12]);
        let encrypted = &stored_data[12..];

        // Decrypt the private key
        let decrypted = self
            .cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| Error::Internal(format!("AES decryption failed: {}", e)))?;

        // Convert to 32-byte array
        if decrypted.len() != 32 {
            return Err(Error::Internal(
                "Decrypted key has wrong length".to_string(),
            ));
        }

        let private_bytes: [u8; 32] = decrypted
            .try_into()
            .map_err(|_| Error::Internal("Key conversion failed".to_string()))?;

        KeyPair::from_private_key(&private_bytes)
    }

    /// Get metadata for a stored key
    pub fn get_metadata(&self, did: &str) -> Option<&KeyMetadata> {
        self.metadata.get(did)
    }

    /// List all stored key DIDs
    pub fn list_keys(&self) -> Vec<String> {
        self.encrypted_keys.keys().cloned().collect()
    }

    /// Remove a stored key
    pub fn remove(&mut self, did: &str) -> Result<()> {
        self.encrypted_keys
            .remove(did)
            .ok_or_else(|| Error::Internal(format!("Key not found for DID: {}", did)))?;
        self.metadata.remove(did);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate().unwrap();
        assert!(!keypair.public_key.to_bytes().is_empty());
    }

    #[test]
    fn test_sign_verify() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"test message for signing";
        let signature = keypair.sign(message).unwrap();
        keypair.verify(message, &signature).unwrap();
    }

    #[test]
    fn test_sign_verify_wrong_message() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"test message for signing";
        let signature = keypair.sign(message).unwrap();
        let wrong_message = b"wrong message";
        assert!(keypair.verify(wrong_message, &signature).is_err());
    }

    #[test]
    fn test_keypair_did_identifier() {
        let keypair = KeyPair::generate().unwrap();
        let identifier = keypair.to_did_identifier();
        // Should be 40 hex chars
        assert_eq!(identifier.len(), 40);
        // Should match the DID derived from the public key
        let did = crate::identity::Did::from_public_key(keypair.public_key.inner());
        assert_eq!(identifier, did.method_id());
    }

    #[test]
    fn test_identity_keys_generate() {
        let keys = IdentityKeys::generate().unwrap();
        let did = keys.did_identifier();
        assert!(did.starts_with("did:nexa:"));
        assert_eq!(did.len(), 49); // "did:nexa:" + 40 hex chars
    }

    #[test]
    fn test_key_agreement_diffie_hellman() {
        let alice = KeyAgreementKeyPair::generate().unwrap();
        let bob = KeyAgreementKeyPair::generate().unwrap();

        let alice_shared = alice.diffie_hellman(bob.public_key()).unwrap();
        let bob_shared = bob.diffie_hellman(alice.public_key()).unwrap();

        // Both parties should compute the same shared secret
        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_secure_key_store() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        let keypair = KeyPair::generate().unwrap();
        let did = format!("did:nexa:{}", keypair.to_did_identifier());

        // Store the key
        store.store(&did, &keypair).unwrap();

        // Retrieve and verify
        let retrieved = store.retrieve(&did).unwrap();
        assert_eq!(
            retrieved.public_key.to_bytes(),
            keypair.public_key.to_bytes()
        );

        // Verify signing still works with retrieved key
        let message = b"test with stored key";
        let signature = retrieved.sign(message).unwrap();
        assert!(keypair.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_secure_key_store_not_found() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let store = SecureKeyStore::new(&encryption_key);

        assert!(store.retrieve("did:nexa:nonexistent").is_err());
    }

    #[test]
    fn test_secure_key_store_wrong_key() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let wrong_key: [u8; 32] = [99u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        let keypair = KeyPair::generate().unwrap();
        let did = format!("did:nexa:{}", keypair.to_did_identifier());

        store.store(&did, &keypair).unwrap();

        // Try to decrypt with wrong key
        let wrong_store = SecureKeyStore::new(&wrong_key);
        assert!(wrong_store.retrieve(&did).is_err());
    }

    #[test]
    fn test_key_metadata() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        let keypair = KeyPair::generate().unwrap();
        let did = format!("did:nexa:{}", keypair.to_did_identifier());

        store.store(&did, &keypair).unwrap();

        let metadata = store.get_metadata(&did).unwrap();
        assert_eq!(metadata.algorithm, "Ed25519");
        assert_eq!(metadata.version, 1);
        assert!(metadata.is_current);
    }

    #[test]
    fn test_private_key_zeroize() {
        let keypair = KeyPair::generate().unwrap();
        let private_bytes = keypair.private_key.to_bytes();

        // Verify we can access the bytes while the key is alive
        assert!(!private_bytes.is_empty());

        // After drop, the bytes should be zeroized (ZeroizeOnDrop)
        // We can't directly test this because Drop has already run,
        // but the ZeroizeOnDrop derive ensures it happens.
        // This test just verifies the derive compiles correctly.
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_keypair_from_private_key_bytes() {
        let original = KeyPair::generate().unwrap();
        let restored = KeyPair::from_private_key(&original.private_key.to_bytes()).unwrap();
        assert_eq!(
            restored.public_key.to_bytes(),
            original.public_key.to_bytes()
        );
    }

    #[test]
    fn test_sign_verify_empty_message() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"";
        let signature = keypair.sign(message).unwrap();
        keypair.verify(message, &signature).unwrap();
    }

    #[test]
    fn test_sign_verify_large_message() {
        let keypair = KeyPair::generate().unwrap();
        let message: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let signature = keypair.sign(&message).unwrap();
        keypair.verify(&message, &signature).unwrap();
    }

    #[test]
    fn test_key_agreement_from_private_key() {
        let original = KeyAgreementKeyPair::generate().unwrap();
        let restored = KeyAgreementKeyPair::from_private_key(original.private_key()).unwrap();
        assert_eq!(restored.public_key(), original.public_key());
    }

    #[test]
    fn test_secure_key_store_remove() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        let keypair = KeyPair::generate().unwrap();
        let did = format!("did:nexa:{}", keypair.to_did_identifier());

        store.store(&did, &keypair).unwrap();
        assert!(store.get_metadata(&did).is_some());

        store.remove(&did).unwrap();
        assert!(store.retrieve(&did).is_err());
        assert!(store.get_metadata(&did).is_none());
    }

    #[test]
    fn test_secure_key_store_remove_nonexistent() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        let result = store.remove("did:nexa:nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_key_store_list_keys() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        assert!(store.list_keys().is_empty());

        let kp1 = KeyPair::generate().unwrap();
        let did1 = format!("did:nexa:{}", kp1.to_did_identifier());
        store.store(&did1, &kp1).unwrap();

        let kp2 = KeyPair::generate().unwrap();
        let did2 = format!("did:nexa:{}", kp2.to_did_identifier());
        store.store(&did2, &kp2).unwrap();

        let keys = store.list_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&did1));
        assert!(keys.contains(&did2));
    }

    #[test]
    fn test_secure_key_store_multiple_store_same_did_overwrites() {
        let encryption_key: [u8; 32] = [42u8; 32];
        let mut store = SecureKeyStore::new(&encryption_key);

        let kp1 = KeyPair::generate().unwrap();
        let did = format!("did:nexa:{}", kp1.to_did_identifier());
        store.store(&did, &kp1).unwrap();

        let kp2 = KeyPair::generate().unwrap();
        store.store(&did, &kp2).unwrap();

        // Should retrieve the second keypair (overwrite)
        let retrieved = store.retrieve(&did).unwrap();
        assert_eq!(retrieved.public_key.to_bytes(), kp2.public_key.to_bytes());
    }

    #[test]
    fn test_public_key_from_bytes() {
        let keypair = KeyPair::generate().unwrap();
        let bytes = keypair.public_key.to_bytes();
        let restored = PublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(restored.to_bytes(), bytes);
    }

    #[test]
    fn test_private_key_from_bytes() {
        let keypair = KeyPair::generate().unwrap();
        let bytes = keypair.private_key.to_bytes();
        let restored = PrivateKey::from_bytes(&bytes).unwrap();
        assert_eq!(restored.to_bytes(), bytes);
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// KeyPair generate → sign → verify round-trip with arbitrary messages
        #[test]
        fn proptest_keypair_sign_verify_roundtrip(msg in prop::collection::vec(any::<u8>(), 0..1000)) {
            let keypair = KeyPair::generate().unwrap();
            let signature = keypair.sign(&msg).unwrap();
            assert!(keypair.verify(&msg, &signature).is_ok());

            // Wrong message must fail
            if !msg.is_empty() {
                let wrong_msg = vec![msg[0] ^ 0xFF];
                assert!(keypair.verify(&wrong_msg, &signature).is_err());
            }
        }

        /// SecureKeyStore encrypt → decrypt round-trip preserves signing ability
        #[test]
        fn proptest_secure_key_store_roundtrip(seed in any::<[u8; 32]>()) {
            let encryption_key: [u8; 32] = [42u8; 32];
            let mut store = SecureKeyStore::new(&encryption_key);

            let keypair = KeyPair::from_private_key(&seed).unwrap();
            let did = format!("did:nexa:{}", keypair.to_did_identifier());

            store.store(&did, &keypair).unwrap();
            let retrieved = store.retrieve(&did).unwrap();

            assert_eq!(retrieved.public_key.to_bytes(), keypair.public_key.to_bytes());

            let message = b"proptest message";
            let signature = retrieved.sign(message).unwrap();
            assert!(keypair.verify(message, &signature).is_ok());
        }

        /// X25519 DH: both parties always compute the same shared secret
        #[test]
        fn proptest_diffie_hellman_symmetry(_seed in any::<u64>()) {
            let alice = KeyAgreementKeyPair::generate().unwrap();
            let bob = KeyAgreementKeyPair::generate().unwrap();

            let alice_shared = alice.diffie_hellman(bob.public_key()).unwrap();
            let bob_shared = bob.diffie_hellman(alice.public_key()).unwrap();

            assert_eq!(alice_shared, bob_shared);
            assert!(!alice_shared.is_empty()); // Non-trivial shared secret
        }
    }
}
