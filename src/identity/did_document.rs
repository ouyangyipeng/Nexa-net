//! DID Document implementation
//!
//! Implements the W3C DID Document specification with Nexa-specific extensions.
//! A DID Document describes the public keys, services, and verification methods
//! associated with a DID. It is the machine-readable representation of an identity.
//!
//! # W3C DID Document Structure
//!
//! Per the W3C DID Core specification and Nexa-net IDENTITY_LAYER.md,
//! a DID Document contains:
//! - `@context`: W3C DID context + Nexa context URIs
//! - `id`: The DID identifier
//! - `controller`: The controller DID (typically self-controlled)
//! - `verificationMethod`: Ed25519 signing key + X25519 key agreement key
//! - `authentication`: Reference to signing verification method
//! - `keyAgreement`: X25519 key for Diffie-Hellman key exchange
//! - `service`: NexaProxyEndpoint service

use crate::error::Result;
use crate::identity::{Did, IdentityKeys};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

/// DID Document as per W3C DID specification with Nexa extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    /// DID context URIs
    #[serde(rename = "@context")]
    pub context: Vec<String>,

    /// DID identifier
    pub id: String,

    /// Controller DID (typically self-controlled)
    pub controller: String,

    /// Verification methods (Ed25519 signing + X25519 key agreement)
    pub verification_method: Vec<VerificationMethod>,

    /// Authentication method references
    pub authentication: Vec<String>,

    /// Key agreement methods (X25519 for DH key exchange)
    pub key_agreement: Vec<VerificationMethod>,

    /// Service endpoints
    pub service: Vec<ServiceEndpoint>,

    /// Created timestamp
    pub created: String,

    /// Updated timestamp
    pub updated: String,
}

/// Verification method (public key entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    /// Method ID (e.g., "did:nexa:abc#key-1")
    pub id: String,

    /// Key type (e.g., "Ed25519VerificationKey2020")
    #[serde(rename = "type")]
    pub key_type: String,

    /// Controller DID
    pub controller: String,

    /// Public key in multibase format (z-prefix + base64)
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

/// Service endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service ID (e.g., "did:nexa:abc#nexa-proxy")
    pub id: String,

    /// Service type
    #[serde(rename = "type")]
    pub service_type: String,

    /// Service endpoint URL
    pub service_endpoint: String,
}

impl DidDocument {
    /// Create a new DID Document from a DID and Ed25519 public key
    ///
    /// This creates a minimal DID Document with only the Ed25519 signing key.
    /// Use `from_identity_keys()` for the full document with key agreement.
    pub fn new(did: &Did, public_key: &VerifyingKey) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let key_id = format!("{}#key-1", did.as_str());

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
                "https://nexa-net.org/ns/did/v1".to_string(),
            ],
            id: did.as_str().to_string(),
            controller: did.as_str().to_string(),
            verification_method: vec![VerificationMethod {
                id: key_id.clone(),
                key_type: "Ed25519VerificationKey2020".to_string(),
                controller: did.as_str().to_string(),
                public_key_multibase: multibase_encode_ed25519(public_key.to_bytes()),
            }],
            authentication: vec![key_id],
            key_agreement: vec![],
            service: vec![],
            created: now.clone(),
            updated: now,
        }
    }

    /// Create a full DID Document from IdentityKeys
    ///
    /// This includes both the Ed25519 signing key and the X25519
    /// key agreement key, as specified in IDENTITY_LAYER.md.
    pub fn from_identity_keys(did: &Did, keys: &IdentityKeys) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let did_str = did.as_str();
        let signing_key_id = format!("{}#key-1", did_str);
        let key_agreement_id = format!("{}#key-agreement-1", did_str);

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
                "https://nexa-net.org/ns/did/v1".to_string(),
            ],
            id: did_str.to_string(),
            controller: did_str.to_string(),
            verification_method: vec![
                VerificationMethod {
                    id: signing_key_id.clone(),
                    key_type: "Ed25519VerificationKey2020".to_string(),
                    controller: did_str.to_string(),
                    public_key_multibase: multibase_encode_ed25519(
                        keys.signing.public_key().to_bytes(),
                    ),
                },
                VerificationMethod {
                    id: key_agreement_id.clone(),
                    key_type: "X25519KeyAgreementKey2020".to_string(),
                    controller: did_str.to_string(),
                    public_key_multibase: multibase_encode_x25519(*keys.key_agreement.public_key()),
                },
            ],
            authentication: vec![signing_key_id],
            key_agreement: vec![VerificationMethod {
                id: key_agreement_id,
                key_type: "X25519KeyAgreementKey2020".to_string(),
                controller: did_str.to_string(),
                public_key_multibase: multibase_encode_x25519(*keys.key_agreement.public_key()),
            }],
            service: vec![],
            created: now.clone(),
            updated: now,
        }
    }

    /// Add a NexaProxyEndpoint service
    pub fn add_nexa_proxy_service(&mut self, endpoint: &str) {
        let service_id = format!("{}#nexa-proxy", self.id);
        self.service.push(ServiceEndpoint {
            id: service_id,
            service_type: "NexaProxyEndpoint".to_string(),
            service_endpoint: endpoint.to_string(),
        });
        self.updated = chrono::Utc::now().to_rfc3339();
    }

    /// Add a generic service endpoint
    pub fn add_service(&mut self, service_type: &str, endpoint: &str) {
        let service_id = format!("{}#service-{}", self.id, self.service.len() + 1);
        self.service.push(ServiceEndpoint {
            id: service_id,
            service_type: service_type.to_string(),
            service_endpoint: endpoint.to_string(),
        });
        self.updated = chrono::Utc::now().to_rfc3339();
    }

    /// Get the Ed25519 verification method
    pub fn signing_key_method(&self) -> Option<&VerificationMethod> {
        self.verification_method
            .iter()
            .find(|m| m.key_type == "Ed25519VerificationKey2020")
    }

    /// Get the X25519 key agreement method
    pub fn key_agreement_method(&self) -> Option<&VerificationMethod> {
        self.key_agreement.first()
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

/// Encode Ed25519 public key bytes as multibase (z-prefix + base64)
fn multibase_encode_ed25519(key_bytes: [u8; 32]) -> String {
    format!(
        "z{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_bytes)
    )
}

/// Encode X25519 public key bytes as multibase (z-prefix + base64)
fn multibase_encode_x25519(key_bytes: [u8; 32]) -> String {
    format!(
        "z{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_bytes)
    )
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
        assert_eq!(doc.controller, did.as_str());
        assert!(!doc.verification_method.is_empty());
        assert!(!doc.authentication.is_empty());
        assert_eq!(doc.context.len(), 3);
    }

    #[test]
    fn test_did_document_from_identity_keys() {
        let keys = IdentityKeys::generate().unwrap();
        let did = Did::from_public_key(keys.signing.public_key().inner());

        let doc = DidDocument::from_identity_keys(&did, &keys);

        assert_eq!(doc.id, did.as_str());
        assert_eq!(doc.controller, did.as_str());
        // Should have 2 verification methods: Ed25519 + X25519
        assert_eq!(doc.verification_method.len(), 2);
        assert!(!doc.authentication.is_empty());
        assert!(!doc.key_agreement.is_empty());

        // Check signing key method
        let signing_method = doc.signing_key_method().unwrap();
        assert_eq!(signing_method.key_type, "Ed25519VerificationKey2020");

        // Check key agreement method
        let ka_method = doc.key_agreement_method().unwrap();
        assert_eq!(ka_method.key_type, "X25519KeyAgreementKey2020");
    }

    #[test]
    fn test_did_document_json_roundtrip() {
        let keys = IdentityKeys::generate().unwrap();
        let did = Did::from_public_key(keys.signing.public_key().inner());

        let doc = DidDocument::from_identity_keys(&did, &keys);
        let json = doc.to_json().unwrap();

        let parsed = DidDocument::from_json(&json).unwrap();
        assert_eq!(parsed.id, doc.id);
        assert_eq!(parsed.controller, doc.controller);
        assert_eq!(
            parsed.verification_method.len(),
            doc.verification_method.len()
        );
        assert_eq!(parsed.key_agreement.len(), doc.key_agreement.len());
    }

    #[test]
    fn test_add_nexa_proxy_service() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);

        let mut doc = DidDocument::new(&did, &verifying_key);
        doc.add_nexa_proxy_service("https://proxy.nexa.net:7070");

        assert_eq!(doc.service.len(), 1);
        assert_eq!(doc.service[0].service_type, "NexaProxyEndpoint");
        assert_eq!(
            doc.service[0].service_endpoint,
            "https://proxy.nexa.net:7070"
        );
    }

    #[test]
    fn test_multibase_encoding() {
        let test_bytes: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];
        let encoded = multibase_encode_ed25519(test_bytes);
        assert!(encoded.starts_with("z"));
        // The encoded part after 'z' should be valid base64
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encoded[1..])
                .unwrap();
        assert_eq!(decoded.len(), 32);
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_did_document_minimal_has_no_key_agreement() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);

        // DidDocument::new (minimal) has no key agreement
        let doc = DidDocument::new(&did, &verifying_key);
        assert!(doc.key_agreement.is_empty());
        assert!(doc.key_agreement_method().is_none());
    }

    #[test]
    fn test_did_document_empty_services_initially() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);

        let doc = DidDocument::new(&did, &verifying_key);
        assert!(doc.service.is_empty());
    }

    #[test]
    fn test_add_generic_service() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let did = Did::from_public_key(&verifying_key);

        let mut doc = DidDocument::new(&did, &verifying_key);
        doc.add_service("MessagingService", "https://msg.nexa.net");

        assert_eq!(doc.service.len(), 1);
        assert_eq!(doc.service[0].service_type, "MessagingService");
        assert_eq!(doc.service[0].service_endpoint, "https://msg.nexa.net");
    }

    #[test]
    fn test_did_document_json_roundtrip_preserves_services() {
        let keys = IdentityKeys::generate().unwrap();
        let did = Did::from_public_key(keys.signing.public_key().inner());

        let mut doc = DidDocument::from_identity_keys(&did, &keys);
        doc.add_nexa_proxy_service("https://proxy.nexa.net:7070");
        doc.add_service("MessagingService", "https://msg.nexa.net");

        let json = doc.to_json().unwrap();
        let parsed = DidDocument::from_json(&json).unwrap();
        assert_eq!(parsed.service.len(), 2);
        assert_eq!(parsed.service[0].service_type, "NexaProxyEndpoint");
        assert_eq!(parsed.service[1].service_type, "MessagingService");
    }

    #[test]
    fn test_did_document_invalid_json_deserialization() {
        let result = DidDocument::from_json("{invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_document_empty_json_deserialization() {
        let result = DidDocument::from_json("{}");
        // Empty JSON should still deserialize with defaults/nulls
        // Required fields are missing, so this should fail
        assert!(result.is_err() || result.unwrap().id.is_empty());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// DID Document JSON round-trip preserves id and controller
        #[test]
        fn proptest_did_document_json_roundtrip(
            method_id in "[a-zA-Z0-9]{10,40}",
            service_endpoint in "[a-zA-Z0-9:/._-]{5,50}",
        ) {
            let keys = IdentityKeys::generate().unwrap();
            let did = Did::from_public_key(keys.signing.public_key().inner());

            let mut doc = DidDocument::from_identity_keys(&did, &keys);
            doc.add_nexa_proxy_service(&service_endpoint);

            let json = doc.to_json().unwrap();
            let parsed = DidDocument::from_json(&json).unwrap();

            assert_eq!(parsed.id, doc.id);
            assert_eq!(parsed.controller, doc.controller);
            assert_eq!(parsed.verification_method.len(), doc.verification_method.len());
            assert_eq!(parsed.key_agreement.len(), doc.key_agreement.len());
            assert_eq!(parsed.service.len(), doc.service.len());
        }
    }
}
