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
//! ```rust,no_run
//! use nexa_net::identity::{Did, DidDocument, KeyPair};
//!
//! // Generate a new DID
//! let keypair = KeyPair::generate()?;
//! let did = Did::from_keypair(&keypair);
//!
//! // Create DID Document
//! let document = DidDocument::new(&did, &keypair.public_key());
//!
//! // Sign and verify
//! let message = b"Hello, Nexa-net!";
//! let signature = keypair.sign(message)?;
//! assert!(keypair.verify(message, &signature)?);
//! # Ok::<(), nexa_net::Error>(())
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