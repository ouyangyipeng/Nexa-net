//! Layer 1: Identity & Zero-Trust Network Layer
//!
//! This module implements decentralized identity (DID) and zero-trust authentication
//! for Nexa-net agents.
//!
//! # Components
//!
//! - **DID**: Nexa-DID generation, parsing, and resolution
//! - **DID Document**: Identity document management
//! - **Key Management**: Cryptographic key generation, storage, and rotation
//! - **mTLS**: Mutual TLS authentication
//! - **Credential**: Verifiable Credentials (VC) issuance and verification
//! - **Trust Anchor**: Trust anchor and governance
//!
//! # Example
//!
//! ```rust,ignore
//! use nexa_net::identity::{Did, DidDocument, IdentityKeys};
//!
//! // Generate a new identity
//! let identity = IdentityKeys::generate().unwrap();
//! let did = Did::new("did:nexa:alice");
//!
//! // Create DID Document
//! let document = DidDocument::new(&did, &identity.signing_key.public_key());
//! ```

pub mod did;
pub mod did_document;
pub mod key_management;
pub mod credential;
pub mod trust_anchor;
pub mod resolver;

// Re-exports
pub use did::Did;
pub use did_document::DidDocument;
pub use key_management::{KeyPair, PublicKey, PrivateKey, KeyAgreementKeyPair, IdentityKeys};
pub use credential::{VerifiableCredential, CredentialClaim};
pub use trust_anchor::TrustAnchor;
pub use resolver::{DidResolver, DidResolutionResult};