//! DID Document implementation
//!
//! DID Document describes the public keys, services, and verification methods
//! associated with a DID.

use crate::error::Result;
use crate::identity::Did;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

/// DID Document as per W3C DID specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    /// DID context
    #[serde(rename = "@context")]
    pub context: Vec<String>,

    /// DID identifier
    pub id: String,

    /// Verification methods (public keys)
    pub verification_method: Vec<VerificationMethod>,

    /// Authentication methods
    pub authentication: Vec<String>,

    /// Service endpoints
    pub service: Vec<ServiceEndpoint>,

    /// Created timestamp
    pub created: String,

    /// Updated timestamp
    pub updated: String,
}

/// Verification method (public key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    /// Method ID
    pub id: String,

    /// Key type
    #[serde(rename = "type")]
    pub key_type: String,

    /// Controller DID
    pub controller: String,

    /// Public key in base58 or multibase format
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

/// Service endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service ID
    pub id: String,

    /// Service type
    #[serde(rename = "type")]
    pub service_type: String,

    /// Service endpoint URL
    pub service_endpoint: String,
}

impl DidDocument {
    /// Create a new DID Document
    pub fn new(did: &Did, public_key: &VerifyingKey) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let key_id = format!("{}#key-1", did.as_str());

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: did.as_str().to_string(),
            verification_method: vec![VerificationMethod {
                id: key_id.clone(),
                key_type: "Ed25519VerificationKey2020".to_string(),
                controller: did.as_str().to_string(),
                public_key_multibase: format!(
                    "z{}",
                    base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        public_key.to_bytes()
                    )
                ),
            }],
            authentication: vec![key_id],
            service: vec![],
            created: now.clone(),
            updated: now,
        }
    }

    /// Add a service endpoint
    pub fn add_service(&mut self, service_type: &str, endpoint: &str) {
        let service_id = format!("{}#service-{}", self.id, self.service.len() + 1);
        self.service.push(ServiceEndpoint {
            id: service_id,
            service_type: service_type.to_string(),
            service_endpoint: endpoint.to_string(),
        });
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_did_document_creation() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);

        let doc = DidDocument::new(&did, &verifying_key);

        assert_eq!(doc.id, did.as_str());
        assert!(!doc.verification_method.is_empty());
        assert!(!doc.authentication.is_empty());
    }

    #[test]
    fn test_did_document_json() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);

        let doc = DidDocument::new(&did, &verifying_key);
        let json = doc.to_json().unwrap();

        let parsed = DidDocument::from_json(&json).unwrap();
        assert_eq!(parsed.id, doc.id);
    }
}
