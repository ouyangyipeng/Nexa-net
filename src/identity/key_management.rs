//! Key management for Nexa-net
//!
//! Provides cryptographic key generation, storage, and operations.
//! Supports Ed25519 for signing and X25519 for key agreement.

use crate::error::{Error, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

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
        let verifying_key = VerifyingKey::from_bytes(bytes)
            .map_err(|e| Error::KeyGeneration(e.to_string()))?;
        Ok(Self(verifying_key))
    }
    
    /// Get the inner key
    pub fn inner(&self) -> &VerifyingKey {
        &self.0
    }
}

/// Private key wrapper for Ed25519
#[derive(Debug, Clone)]
pub struct PrivateKey(SigningKey);

impl PrivateKey {
    /// Get the raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
    
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(bytes);
        Ok(Self(signing_key))
    }
    
    /// Get the inner key
    pub fn inner(&self) -> &SigningKey {
        &self.0
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
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        Ok(Self {
            private_key: PrivateKey(signing_key),
            public_key: PublicKey(verifying_key),
        })
    }
    
    /// Create from existing private key bytes
    pub fn from_private_key(bytes: &[u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        
        Ok(Self {
            private_key: PrivateKey(signing_key),
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
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Result<Signature> {
        Ok(self.private_key.0.sign(message))
    }
    
    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        self.public_key.0
            .verify(message, signature)
            .map_err(|e| Error::SignatureVerification(e.to_string()))
    }
}

// ============================================================================
// X25519 Key Agreement Keys
// ============================================================================

/// Key agreement key pair (X25519)
#[derive(Debug, Clone)]
pub struct KeyAgreementKeyPair {
    /// Private key for key agreement
    private_key: [u8; 32],
    /// Public key for key agreement
    public_key: [u8; 32],
}

impl KeyAgreementKeyPair {
    /// Generate a new X25519 key pair
    pub fn generate() -> Result<Self> {
        let secret = StaticSecret::random_from_rng(OsRng);
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

impl Drop for KeyAgreementKeyPair {
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

// ============================================================================
// Combined Identity Keys
// ============================================================================

/// Complete identity key set
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
}

/// Key storage (encrypted at rest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStorage {
    /// Encrypted private key
    encrypted_private_key: Vec<u8>,
    
    /// Public key (not encrypted)
    public_key: Vec<u8>,
    
    /// Encryption nonce
    nonce: Vec<u8>,
}

impl KeyStorage {
    /// Create encrypted key storage
    pub fn new(keypair: &KeyPair, encryption_key: &[u8]) -> Result<Self> {
        // TODO: Implement actual encryption with AES-256-GCM
        // For now, store unencrypted (NOT for production)
        Ok(Self {
            encrypted_private_key: keypair.private_key.to_bytes().to_vec(),
            public_key: keypair.public_key.to_bytes().to_vec(),
            nonce: vec![0u8; 12],
        })
    }
    
    /// Decrypt and recover key pair
    pub fn decrypt(&self, encryption_key: &[u8]) -> Result<KeyPair> {
        // TODO: Implement actual decryption
        let bytes: [u8; 32] = self.encrypted_private_key
            .as_slice()
            .try_into()
            .map_err(|_| Error::KeyGeneration("Invalid key length".to_string()))?;
        KeyPair::from_private_key(&bytes)
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
        let message = b"Hello, Nexa-net!";
        
        let signature = keypair.sign(message).unwrap();
        assert!(keypair.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_sign_verify_wrong_message() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"Hello, Nexa-net!";
        let wrong_message = b"Wrong message";
        
        let signature = keypair.sign(message).unwrap();
        assert!(keypair.verify(wrong_message, &signature).is_err());
    }
}