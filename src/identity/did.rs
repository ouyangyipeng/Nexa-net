//! DID (Decentralized Identifier) implementation
//!
//! Nexa-DID follows W3C DID specification with Nexa-specific method.

use crate::error::{Error, Result};
use crate::types::Did as DidType;
use ed25519_dalek::VerifyingKey;
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
        let hash = Sha256::digest(public_key_bytes);
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

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_did_parse_empty_string() {
        let result = Did::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_parse_missing_prefix() {
        let result = Did::parse("nexa:abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_parse_wrong_method() {
        let result = Did::parse("did:other:abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_parse_only_prefix() {
        // "did:nexa:" with no method-specific ID — still valid per current impl
        let result = Did::parse("did:nexa:");
        assert!(result.is_ok());
        let did = result.unwrap();
        assert_eq!(did.method_id(), "");
    }

    #[test]
    fn test_did_method_id_extraction() {
        let did_str = "did:nexa:abc123def456";
        let did = Did::parse(did_str).unwrap();
        assert_eq!(did.method_id(), "abc123def456");
    }

    #[test]
    fn test_did_display_format() {
        let did_str = "did:nexa:abc123";
        let did = Did::parse(did_str).unwrap();
        assert_eq!(format!("{}", did), did_str);
    }

    #[test]
    fn test_did_from_public_key_deterministic() {
        let signing_key_bytes: [u8; 32] = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let verifying_key = signing_key.verifying_key();

        let did1 = Did::from_public_key(&verifying_key);
        let did2 = Did::from_public_key(&verifying_key);
        assert_eq!(did1.as_str(), did2.as_str());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Any string starting with "did:nexa:" should parse successfully
        #[test]
        fn proptest_did_parse_valid_prefix(method_id in "[a-zA-Z0-9]{1,40}") {
            let did_str = format!("did:nexa:{}", method_id);
            let did = Did::parse(&did_str).unwrap();
            assert_eq!(did.as_str(), did_str);
            assert_eq!(did.method_id(), method_id);
        }

        /// Any string NOT starting with "did:nexa:" should fail to parse
        #[test]
        fn proptest_did_parse_invalid_prefix(s in "[^d][a-zA-Z0-9:_]{0,50}") {
            let result = Did::parse(&s);
            assert!(result.is_err());
        }

        /// DID derived from public key always has valid format
        #[test]
        fn proptest_did_from_public_key_format(seed in any::<[u8; 32]>()) {
            let signing_key = SigningKey::from_bytes(&seed);
            let verifying_key = signing_key.verifying_key();
            let did = Did::from_public_key(&verifying_key);

            assert!(did.as_str().starts_with("did:nexa:"));
            // Method ID should be 40 hex chars (SHA256 truncated to 20 bytes → 40 hex)
            assert_eq!(did.method_id().len(), 40);
            // Method ID should be valid hex
            for c in did.method_id().chars() {
                assert!(c.is_ascii_hexdigit());
            }
        }
    }
}
