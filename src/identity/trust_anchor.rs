//! Trust Anchor implementation
//!
//! Trust anchors are trusted entities that can issue credentials
//! and participate in governance.

use crate::error::Result;
use crate::identity::{Did, VerifiableCredential};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Trust anchor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAnchor {
    /// Trust anchor DID
    pub did: String,
    
    /// Trust anchor name
    pub name: String,
    
    /// Trust anchor public key
    pub public_key: String,
    
    /// Trust level
    pub trust_level: u8,
    
    /// Supported credential types
    pub supported_credentials: Vec<String>,
}

impl TrustAnchor {
    /// Create a new trust anchor
    pub fn new(did: &Did, name: &str, public_key: &str) -> Self {
        Self {
            did: did.as_str().to_string(),
            name: name.to_string(),
            public_key: public_key.to_string(),
            trust_level: 1,
            supported_credentials: vec!["VerifiableCredential".to_string()],
        }
    }
    
    /// Check if this anchor can issue a credential type
    pub fn can_issue(&self, credential_type: &str) -> bool {
        self.supported_credentials.contains(&credential_type.to_string())
    }
}

/// Trust anchor registry
#[derive(Debug, Clone, Default)]
pub struct TrustAnchorRegistry {
    /// Registered trust anchors
    anchors: HashSet<String>,
}

impl TrustAnchorRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a trust anchor
    pub fn register(&mut self, anchor: &TrustAnchor) {
        self.anchors.insert(anchor.did.clone());
    }
    
    /// Check if a DID is a trusted anchor
    pub fn is_trusted(&self, did: &str) -> bool {
        self.anchors.contains(did)
    }
    
    /// Remove a trust anchor
    pub fn remove(&mut self, did: &str) {
        self.anchors.remove(did);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_anchor_creation() {
        let did = Did::parse("did:nexa:anchor123").unwrap();
        let anchor = TrustAnchor::new(&did, "Test Anchor", "public_key_hex");
        
        assert_eq!(anchor.did, did.as_str());
        assert_eq!(anchor.name, "Test Anchor");
    }

    #[test]
    fn test_trust_anchor_registry() {
        let mut registry = TrustAnchorRegistry::new();
        let did = Did::parse("did:nexa:anchor123").unwrap();
        let anchor = TrustAnchor::new(&did, "Test Anchor", "public_key_hex");
        
        registry.register(&anchor);
        assert!(registry.is_trusted(did.as_str()));
        
        registry.remove(did.as_str());
        assert!(!registry.is_trusted(did.as_str()));
    }
}