//! Layer 1: Identity & Zero-Trust Network Layer
//!
//! This module implements decentralized identity (DID) and zero-trust authentication
//! for Nexa-net agents.
//!
//! # Components
//!
//! - **DID**: Nexa-DID generation, parsing, and resolution
//! - **DID Document**: Identity document management (W3C DID Core spec)
//! - **Key Management**: Cryptographic key generation, encrypted storage, and zeroize
//! - **mTLS**: Mutual TLS authentication with DID-derived certificates
//! - **Credential**: Verifiable Credentials (VC) with Ed25519 signing and verification
//! - **Trust Anchor**: Trust anchor registry and governance
//! - **Resolver**: DID resolution with caching and verification
//!
//! # Example
//!
//! ```rust,ignore
//! use nexa_net::identity::{Did, DidDocument, IdentityKeys};
//!
//! // Generate a new identity
//! let identity = IdentityKeys::generate().unwrap();
//! let did = Did::from_public_key(identity.signing.public_key().inner());
//!
//! // Create DID Document with full key material
//! let document = DidDocument::from_identity_keys(&did, &identity);
//! ```

pub mod credential;
pub mod did;
pub mod did_document;
pub mod key_management;
pub mod mtls;
pub mod resolver;
pub mod trust_anchor;

// Re-exports
pub use credential::{CredentialClaim, VerifiableCredential};
pub use did::Did;
pub use did_document::{DidDocument, ServiceEndpoint, VerificationMethod};
pub use key_management::{
    IdentityKeys, KeyAgreementKeyPair, KeyMetadata, KeyPair, PrivateKey, PublicKey, SecureKeyStore,
};
pub use mtls::{generate_self_signed_cert, verify_cert_matches_did, DidCertificate, MtlsConfig};
pub use resolver::{DidResolutionResult, DidResolver, ResolutionSource};
pub use trust_anchor::{TrustAnchor, TrustAnchorRegistry};
