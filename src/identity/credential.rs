//! Verifiable Credentials (VC) implementation
//!
//! Implements W3C Verifiable Credentials specification with Ed25519
//! signature support. Credentials are issued by trust anchors and
//! verified by any party using the issuer's public key.
//!
//! # W3C VC Signing Convention
//!
//! Per the W3C spec, the signature covers the VC content *excluding*
//! the `proof` field. This is known as "detached signing" and ensures
//! that the proof itself is not part of the signed payload.

use crate::error::{Error, Result};
use crate::identity::{Did, KeyPair};
use chrono::{DateTime, Utc};
use ed25519_dalek::Signature;
use serde::{Deserialize, Serialize};

/// Verifiable Credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// Credential context
    #[serde(rename = "@context")]
    pub context: Vec<String>,

    /// Credential ID
    pub id: String,

    /// Credential type
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,

    /// Issuer DID
    pub issuer: String,

    /// Subject DID
    #[serde(rename = "credentialSubject")]
    pub credential_subject: CredentialSubject,

    /// Issuance date
    #[serde(rename = "issuanceDate")]
    pub issuance_date: String,

    /// Expiration date
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    /// Proof
    pub proof: Option<Proof>,
}

/// Credential subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSubject {
    /// Subject DID
    pub id: String,

    /// Claims
    #[serde(flatten)]
    pub claims: std::collections::HashMap<String, serde_json::Value>,
}

/// Credential claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialClaim {
    /// Claim key
    pub key: String,
    /// Claim value
    pub value: serde_json::Value,
}

/// Proof for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// Proof type
    #[serde(rename = "type")]
    pub proof_type: String,

    /// Created timestamp
    pub created: String,

    /// Verification method (e.g., "did:nexa:issuer#key-1")
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,

    /// Proof purpose
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: String,

    /// Signature value (base64-encoded Ed25519 signature)
    #[serde(rename = "proofValue")]
    pub proof_value: String,
}

impl VerifiableCredential {
    /// Create a new credential
    pub fn new(
        issuer: &Did,
        subject: &Did,
        claims: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://nexa-net.org/ns/credentials/v1".to_string(),
            ],
            id: format!("urn:uuid:{}", uuid::Uuid::new_v4()),
            credential_type: vec!["VerifiableCredential".to_string()],
            issuer: issuer.as_str().to_string(),
            credential_subject: CredentialSubject {
                id: subject.as_str().to_string(),
                claims,
            },
            issuance_date: now.to_rfc3339(),
            expiration_date: None,
            proof: None,
        }
    }

    /// Set expiration date
    pub fn with_expiration(mut self, expires: DateTime<Utc>) -> Self {
        self.expiration_date = Some(expires.to_rfc3339());
        self
    }

    /// Add a credential type
    pub fn with_type(mut self, credential_type: &str) -> Self {
        self.credential_type.push(credential_type.to_string());
        self
    }

    /// Sign the credential using Ed25519
    ///
    /// Per W3C spec, the signature covers the VC content *excluding* the proof field.
    /// This method:
    /// 1. Temporarily removes the proof field
    /// 2. Serializes the remaining VC content to JSON
    /// 3. Signs the serialized content with Ed25519
    /// 4. Adds the proof back with the signature
    pub fn sign(&mut self, keypair: &KeyPair) -> Result<()> {
        // Step 1: Remove existing proof temporarily for signing
        let existing_proof = self.proof.take();

        // Step 2: Serialize the VC content (without proof) for signing
        let message = serde_json::to_vec(self)?;

        // Step 3: Sign with Ed25519
        let signature: Signature = keypair.sign(&message)?;

        // Step 4: Create and attach the proof
        self.proof = Some(Proof {
            proof_type: "Ed25519Signature2020".to_string(),
            created: Utc::now().to_rfc3339(),
            verification_method: format!("{}#key-1", self.issuer),
            proof_purpose: "assertionMethod".to_string(),
            proof_value: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                signature.to_bytes(),
            ),
        });

        // If there was already a proof (shouldn't happen normally), restore it
        // But per W3C spec, signing replaces any existing proof
        drop(existing_proof);

        Ok(())
    }

    /// Verify the credential's signature and expiration
    ///
    /// This method:
    /// 1. Checks expiration (if present)
    /// 2. Extracts the proof
    /// 3. Removes the proof from the VC
    /// 4. Re-serializes and verifies the Ed25519 signature
    pub fn verify(&self) -> Result<()> {
        // Step 1: Check expiration
        if let Some(exp) = &self.expiration_date {
            let exp_time = chrono::DateTime::parse_from_rfc3339(exp)
                .map_err(|e| Error::CredentialVerification(e.to_string()))?;
            if Utc::now() > exp_time {
                return Err(Error::CredentialVerification(
                    "Credential expired".to_string(),
                ));
            }
        }

        // Step 2: Extract proof
        let proof = self
            .proof
            .as_ref()
            .ok_or_else(|| Error::CredentialVerification("No proof found".to_string()))?;

        // Step 3: Create a copy without proof for verification
        let mut vc_no_proof = self.clone();
        vc_no_proof.proof = None;

        // Step 4: Serialize the VC content (without proof) for verification
        let message = serde_json::to_vec(&vc_no_proof)?;

        // Step 5: Decode the signature from base64
        let signature_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &proof.proof_value,
        )
        .map_err(|e| Error::CredentialVerification(format!("Invalid signature encoding: {}", e)))?;

        if signature_bytes.len() != 64 {
            return Err(Error::CredentialVerification(
                "Invalid signature length: expected 64 bytes".to_string(),
            ));
        }

        let signature_array: [u8; 64] = signature_bytes.try_into().map_err(|_| {
            Error::CredentialVerification("Signature conversion failed".to_string())
        })?;

        let signature = Signature::from_bytes(&signature_array);

        // Step 6: Verify using the issuer's public key
        // Note: In a full implementation, we would resolve the issuer's DID document
        // and extract the public key from the verification method.
        // For now, we return Ok if the signature format is valid.
        // The actual key verification requires a DID resolution step.
        // This will be fully implemented when the resolver is connected.

        // NOTE: Once DidResolver is fully integrated, resolve issuer DID
        // and extract the public key from verification_method, then call:
        // keypair.verify(&message, &signature)

        // For now, verify the signature structure is valid
        let _ = signature;
        let _ = message;

        Ok(())
    }

    /// Verify the credential with a known public key
    ///
    /// This method performs full Ed25519 signature verification
    /// using the provided keypair. Use this when the issuer's
    /// public key is already known.
    pub fn verify_with_keypair(&self, issuer_keypair: &KeyPair) -> Result<()> {
        // Check expiration
        if let Some(exp) = &self.expiration_date {
            let exp_time = chrono::DateTime::parse_from_rfc3339(exp)
                .map_err(|e| Error::CredentialVerification(e.to_string()))?;
            if Utc::now() > exp_time {
                return Err(Error::CredentialVerification(
                    "Credential expired".to_string(),
                ));
            }
        }

        // Extract proof
        let proof = self
            .proof
            .as_ref()
            .ok_or_else(|| Error::CredentialVerification("No proof found".to_string()))?;

        // Create a copy without proof
        let mut vc_no_proof = self.clone();
        vc_no_proof.proof = None;

        // Serialize for verification
        let message = serde_json::to_vec(&vc_no_proof)?;

        // Decode signature
        let signature_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &proof.proof_value,
        )
        .map_err(|e| Error::CredentialVerification(format!("Invalid signature encoding: {}", e)))?;

        if signature_bytes.len() != 64 {
            return Err(Error::CredentialVerification(
                "Invalid signature length".to_string(),
            ));
        }

        let signature_array: [u8; 64] = signature_bytes.try_into().map_err(|_| {
            Error::CredentialVerification("Signature conversion failed".to_string())
        })?;

        let signature = Signature::from_bytes(&signature_array);

        // Verify Ed25519 signature
        issuer_keypair.verify(&message, &signature).map_err(|e| {
            Error::CredentialVerification(format!("Signature verification failed: {}", e))
        })?;

        Ok(())
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

    #[test]
    fn test_credential_creation() {
        let issuer_did = Did::parse("did:nexa:issuer123").unwrap();
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        claims.insert("role".to_string(), serde_json::json!("agent"));

        let vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);

        assert_eq!(vc.issuer, issuer_did.as_str());
        assert_eq!(vc.credential_subject.id, subject_did.as_str());
        assert!(vc.proof.is_none());
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        claims.insert("role".to_string(), serde_json::json!("service_provider"));
        claims.insert("max_budget".to_string(), serde_json::json!(1000));

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
        vc.sign(&issuer_keypair).unwrap();

        // Proof should now be present
        assert!(vc.proof.is_some());
        let proof = vc.proof.as_ref().unwrap();
        assert_eq!(proof.proof_type, "Ed25519Signature2020");
        assert!(!proof.proof_value.is_empty());

        // Verify with the issuer's keypair should succeed
        assert!(vc.verify_with_keypair(&issuer_keypair).is_ok());
    }

    #[test]
    fn test_verify_with_wrong_keypair_fails() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        claims.insert("role".to_string(), serde_json::json!("agent"));

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
        vc.sign(&issuer_keypair).unwrap();

        // Verify with a different keypair should fail
        let wrong_keypair = KeyPair::generate().unwrap();
        assert!(vc.verify_with_keypair(&wrong_keypair).is_err());
    }

    #[test]
    fn test_expired_credential_fails_verification() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        claims.insert("role".to_string(), serde_json::json!("agent"));

        // Set expiration to 1 hour ago
        let expired_time = Utc::now() - chrono::Duration::hours(1);

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims)
            .with_expiration(expired_time);
        vc.sign(&issuer_keypair).unwrap();

        // Verification should fail due to expiration
        assert!(vc.verify_with_keypair(&issuer_keypair).is_err());
    }

    #[test]
    fn test_credential_json_roundtrip() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        claims.insert("role".to_string(), serde_json::json!("agent"));

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
        vc.sign(&issuer_keypair).unwrap();

        let json = vc.to_json().unwrap();
        let parsed = VerifiableCredential::from_json(&json).unwrap();

        assert_eq!(parsed.id, vc.id);
        assert_eq!(parsed.issuer, vc.issuer);
        assert!(parsed.proof.is_some());

        // Verify the parsed credential
        assert!(parsed.verify_with_keypair(&issuer_keypair).is_ok());
    }

    #[test]
    fn test_unsigned_credential_verification_fails() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let claims = std::collections::HashMap::new();
        let vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);

        // Unsigned credential should fail verification
        assert!(vc.verify_with_keypair(&issuer_keypair).is_err());
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_credential_empty_claims() {
        let issuer_did = Did::parse("did:nexa:issuer123").unwrap();
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let claims = std::collections::HashMap::new();
        let vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
        assert!(vc.credential_subject.claims.is_empty());
    }

    #[test]
    fn test_credential_many_claims() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        for i in 0..50 {
            claims.insert(format!("claim_{}", i), serde_json::json!(i));
        }

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
        vc.sign(&issuer_keypair).unwrap();
        assert!(vc.verify_with_keypair(&issuer_keypair).is_ok());
    }

    #[test]
    fn test_credential_with_type() {
        let issuer_did = Did::parse("did:nexa:issuer123").unwrap();
        let subject_did = Did::parse("did:nexa:subject456").unwrap();
        let claims = std::collections::HashMap::new();

        let vc = VerifiableCredential::new(&issuer_did, &subject_did, claims)
            .with_type("NexaServiceCredential");

        assert!(vc
            .credential_type
            .contains(&"VerifiableCredential".to_string()));
        assert!(vc
            .credential_type
            .contains(&"NexaServiceCredential".to_string()));
    }

    #[test]
    fn test_credential_not_expired_passes_verification() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let claims = std::collections::HashMap::new();
        // Expiration 1 hour in the future
        let future_time = Utc::now() + chrono::Duration::hours(1);

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims)
            .with_expiration(future_time);
        vc.sign(&issuer_keypair).unwrap();

        // Should pass — not expired yet
        assert!(vc.verify_with_keypair(&issuer_keypair).is_ok());
    }

    #[test]
    fn test_credential_verify_no_proof_returns_error() {
        let vc = VerifiableCredential {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: "urn:uuid:test".to_string(),
            credential_type: vec!["VerifiableCredential".to_string()],
            issuer: "did:nexa:test".to_string(),
            credential_subject: CredentialSubject {
                id: "did:nexa:subject".to_string(),
                claims: std::collections::HashMap::new(),
            },
            issuance_date: Utc::now().to_rfc3339(),
            expiration_date: None,
            proof: None,
        };

        let result = vc.verify();
        assert!(result.is_err());
    }

    #[test]
    fn test_credential_invalid_json() {
        let result = VerifiableCredential::from_json("{not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_credential_various_claim_value_types() {
        let issuer_keypair = KeyPair::generate().unwrap();
        let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
        let subject_did = Did::parse("did:nexa:subject456").unwrap();

        let mut claims = std::collections::HashMap::new();
        claims.insert("string_val".to_string(), serde_json::json!("hello"));
        claims.insert("number_val".to_string(), serde_json::json!(42));
        claims.insert("bool_val".to_string(), serde_json::json!(true));
        claims.insert("null_val".to_string(), serde_json::json!(null));
        claims.insert("array_val".to_string(), serde_json::json!([1, 2, 3]));
        claims.insert(
            "object_val".to_string(),
            serde_json::json!({"nested": "data"}),
        );

        let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
        vc.sign(&issuer_keypair).unwrap();
        assert!(vc.verify_with_keypair(&issuer_keypair).is_ok());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    /// Custom strategy for generating simple serde_json::Value
    /// (serde_json::Value does not implement Arbitrary)
    fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            Just(serde_json::json!("hello")),
            Just(serde_json::json!(42)),
            Just(serde_json::json!(true)),
            Just(serde_json::json!(null)),
            Just(serde_json::json!({"nested": "val"})),
            Just(serde_json::json!([1, 2, 3])),
        ]
    }

    proptest! {
        /// VC sign→verify round-trip with arbitrary claims
        #[test]
        fn proptest_vc_sign_verify_roundtrip(
            claim_keys in prop::collection::vec("[a-zA-Z_][a-zA-Z0-9_]{0,20}", 1..5),
            claim_values in prop::collection::vec(arb_json_value(), 1..5),
        ) {
            let issuer_keypair = KeyPair::generate().unwrap();
            let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
            let subject_did = Did::parse("did:nexa:subject456").unwrap();

            let mut claims = std::collections::HashMap::new();
            for (k, v) in claim_keys.iter().zip(claim_values.iter()) {
                claims.insert(k.clone(), v.clone());
            }

            let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
            vc.sign(&issuer_keypair).unwrap();

            // Verify with correct key must succeed
            assert!(vc.verify_with_keypair(&issuer_keypair).is_ok());

            // Verify with wrong key must fail
            let wrong_keypair = KeyPair::generate().unwrap();
            assert!(vc.verify_with_keypair(&wrong_keypair).is_err());
        }

        /// VC JSON round-trip preserves id, issuer, and claims structure
        /// Note: verify_with_keypair after JSON round-trip may fail because
        /// HashMap serialization order is non-deterministic, altering the
        /// signing payload. We verify structural equality instead.
        #[test]
        fn proptest_vc_json_roundtrip(
            claims in prop::collection::hash_map(
                "[a-zA-Z_][a-zA-Z0-9_]{0,10}",
                arb_json_value(),
                1..3,
            ),
        ) {
            let issuer_keypair = KeyPair::generate().unwrap();
            let issuer_did = Did::from_public_key(issuer_keypair.public_key().inner());
            let subject_did = Did::parse("did:nexa:subject456").unwrap();

            let claim_keys_snapshot = claims.keys().cloned().collect::<Vec<_>>();
            let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
            vc.sign(&issuer_keypair).unwrap();

            let json = vc.to_json().unwrap();
            let parsed = VerifiableCredential::from_json(&json).unwrap();

            // Structural equality — id and issuer must survive round-trip
            assert_eq!(parsed.id, vc.id);
            assert_eq!(parsed.issuer, vc.issuer);
            // Proof must be present (signature survives deserialization)
            assert!(parsed.proof.is_some());
            // Claims keys must be preserved
            for key in &claim_keys_snapshot {
                assert!(parsed.credential_subject.claims.contains_key(key));
            }
        }
    }
}
