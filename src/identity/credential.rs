//! Verifiable Credentials (VC) implementation
//!
//! Provides credential issuance, verification, and management.

use crate::error::{Error, Result};
use crate::identity::{Did, KeyPair};
use chrono::{DateTime, Utc};
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

    /// Verification method
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,

    /// Proof purpose
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: String,

    /// Signature value
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
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
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

    /// Sign the credential
    pub fn sign(&mut self, keypair: &KeyPair) -> Result<()> {
        let message = serde_json::to_vec(&self)?;
        let signature = keypair.sign(&message)?;

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

        Ok(())
    }

    /// Verify the credential
    pub fn verify(&self) -> Result<()> {
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

        // TODO: Verify signature
        Ok(())
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
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
    }
}
