//! DID (Decentralized Identifier) implementation
//!
//! Nexa-DID follows W3C DID specification with Nexa-specific method.

use crate::error::{Error, Result};
use crate::types::Did as DidType;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// DID identifier with associated key material
#[derive(Debug, Clone)]
pub struct Did {
    /// DID string identifier
    identifier: String,
}

impl Did {
    /// Create a new DID from a public key
    pub fn from_public_key(public_key: &VerifyingKey) -> Self {
        let public_key_bytes = public_key.to_bytes();
        let hash = Sha256::digest(&public_key_bytes);
        let identifier = hex::encode(&hash[..20]);
        Self {
            identifier: format!("did:nexa:{}", identifier),
        }
    }
    
    /// Parse a DID string
    pub fn parse(did: &str) -> Result<Self> {
        if !did.starts_with("did:nexa:") {
            return Err(Error::InvalidDidFormat(did.to_string()));
        }
        Ok(Self {
            identifier: did.to_string(),
        })
    }
    
    /// Get the DID string
    pub fn as_str(&self) -> &str {
        &self.identifier
    }
    
    /// Get the method-specific identifier (the part after "did:nexa:")
    pub fn method_id(&self) -> &str {
        &self.identifier[9..]
    }
}

impl std::fmt::Display for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl From<Did> for DidType {
    fn from(did: Did) -> Self {
        DidType::new(did.identifier)
    }
}

impl From<&Did> for DidType {
    fn from(did: &Did) -> Self {
        DidType::new(&did.identifier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_did_from_public_key() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);
        
        assert!(did.as_str().starts_with("did:nexa:"));
        assert_eq!(did.as_str().len(), 49); // "did:nexa:" + 40 hex chars
    }

    #[test]
    fn test_did_parse() {
        let did_str = "did:nexa:1234567890abcdef1234567890abcdef12345678";
        let did = Did::parse(did_str).unwrap();
        assert_eq!(did.as_str(), did_str);
    }

    #[test]
    fn test_did_parse_invalid() {
        let result = Did::parse("invalid:did");
        assert!(result.is_err());
    }
}