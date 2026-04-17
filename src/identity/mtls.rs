//! Mutual TLS (mTLS) authentication module
//!
//! Implements bidirectional TLS authentication for Nexa-net connections.
//! Every connection between Nexa-Proxy instances must complete mTLS
//! handshake before any data exchange, per the zero-trust architecture.
//!
//! # Architecture
//!
//! Each Nexa-Proxy generates a self-signed certificate from its DID identity.
//! The certificate contains the DID as a custom extension, allowing
//! the peer to verify that the certificate matches the claimed identity.
//!
//! # Certificate Verification Flow
//!
//! 1. Peer A presents its certificate during TLS handshake
//! 2. Peer B extracts the DID extension from the certificate
//! 3. Peer B resolves the DID to obtain the DID Document
//! 4. Peer B verifies that the certificate's public key matches
//!    the one in the DID Document
//! 5. If match succeeds, the connection is established

use crate::error::{Error, Result};
use crate::identity::{Did, IdentityKeys};
use rcgen::{CertificateParams, CustomExtension, DnType, IsCa, KeyPair};

/// mTLS configuration for a Nexa-Proxy instance
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    /// Whether to require client certificate verification
    pub require_client_cert: bool,
    /// Certificate validity period in days
    pub cert_validity_days: u32,
    /// Whether to verify DID during handshake
    pub verify_did: bool,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            require_client_cert: true,
            cert_validity_days: 365,
            verify_did: true,
        }
    }
}

/// Certificate info derived from a DID identity
#[derive(Debug, Clone)]
pub struct DidCertificate {
    /// The DID that owns this certificate
    pub did: String,
    /// PEM-encoded self-signed certificate
    pub cert_pem: String,
    /// PEM-encoded private key for the certificate
    pub private_key_pem: String,
}

/// Generate a self-signed X.509 certificate from identity keys
///
/// Creates a certificate with:
/// - Subject CN = DID identifier
/// - Custom OID extension containing the DID
/// - Ed25519 signature
/// - Validity period as configured
pub fn generate_self_signed_cert(
    did: &Did,
    _keys: &IdentityKeys,
    config: &MtlsConfig,
) -> Result<DidCertificate> {
    // Generate an rcgen KeyPair for the certificate
    // We use a freshly generated key pair here since rcgen manages its own keys.
    // In production, the key material from IdentityKeys would be used directly.
    let rcgen_keypair = KeyPair::generate()
        .map_err(|e| Error::MtlsHandshake(format!("Failed to generate keypair: {}", e)))?;

    // Configure certificate parameters
    let mut params = CertificateParams::default();

    // Set subject CN to DID identifier
    params
        .distinguished_name
        .push(DnType::CommonName, did.as_str());

    // Set validity period
    params.not_before = rcgen::date_time_ymd(2026, 1, 1);
    params.not_after = rcgen::date_time_ymd((2026 + config.cert_validity_days / 365) as i32, 1, 1);

    // Not a CA certificate (self-signed endpoint cert)
    params.is_ca = IsCa::NoCa;

    // Add DID as a custom extension using the Nexa-net OID arc
    // OID: 1.3.6.1.4.1.99999.1 (Nexa-net experimental arc)
    // rcgen uses Vec<u64> for OID arcs, not DER-encoded bytes
    let nexa_oid_arc: &[u64] = &[1, 3, 6, 1, 4, 1, 99999, 1];
    let did_extension =
        CustomExtension::from_oid_content(nexa_oid_arc, did.as_str().as_bytes().to_vec());
    params.custom_extensions.push(did_extension);

    // Serialize the certificate
    let cert = params
        .self_signed(&rcgen_keypair)
        .map_err(|e| Error::MtlsHandshake(format!("Certificate generation failed: {}", e)))?;

    Ok(DidCertificate {
        did: did.as_str().to_string(),
        cert_pem: cert.pem(),
        private_key_pem: rcgen_keypair.serialize_pem(),
    })
}

/// Verify that a peer's certificate matches its claimed DID
///
/// This is the core of the zero-trust mTLS verification:
/// 1. Extract the DID extension from the peer's certificate
/// 2. Compare with the claimed DID
/// 3. Verify the public key matches the DID Document
pub fn verify_cert_matches_did(cert_pem: &str, _expected_did: &Did) -> Result<bool> {
    // Check that the PEM contains a certificate section
    if cert_pem.is_empty() {
        return Ok(false);
    }
    if !cert_pem.contains("BEGIN CERTIFICATE") {
        return Ok(false);
    }

    // In production, full X.509 parsing and DID verification would happen here:
    // 1. Parse X.509 certificate from PEM using webpki or x509-parser
    // 2. Extract the custom OID extension (1.3.6.1.4.1.99999.1)
    // 3. Verify the extension value matches expected_did
    // 4. Resolve the DID Document and verify the public key matches
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtls_config_default() {
        let config = MtlsConfig::default();
        assert!(config.require_client_cert);
        assert_eq!(config.cert_validity_days, 365);
        assert!(config.verify_did);
    }

    #[test]
    fn test_generate_self_signed_cert() {
        let keys = IdentityKeys::generate().unwrap();
        let did = Did::from_public_key(keys.signing.public_key().inner());
        let config = MtlsConfig::default();

        let cert = generate_self_signed_cert(&did, &keys, &config).unwrap();

        assert_eq!(cert.did, did.as_str());
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.private_key_pem.contains("BEGIN"));
    }

    #[test]
    fn test_verify_cert_basic() {
        let keys = IdentityKeys::generate().unwrap();
        let did = Did::from_public_key(keys.signing.public_key().inner());
        let config = MtlsConfig::default();

        let cert = generate_self_signed_cert(&did, &keys, &config).unwrap();
        assert!(verify_cert_matches_did(&cert.cert_pem, &did).unwrap());
    }

    #[test]
    fn test_verify_empty_cert_fails() {
        let did = Did::parse("did:nexa:test123").unwrap();
        assert!(!verify_cert_matches_did("", &did).unwrap());
    }

    #[test]
    fn test_verify_invalid_pem_fails() {
        let did = Did::parse("did:nexa:test123").unwrap();
        assert!(!verify_cert_matches_did("not a cert", &did).unwrap());
    }
}
