//! Layer 2: Semantic Discovery & Capability Routing Layer
//!
//! This module implements semantic service discovery and capability-based routing
//! for Nexa-net agents.
//!
//! # Components
//!
//! - **Capability**: Service capability schema definition and registration
//! - **Vectorizer**: Semantic vectorization using embedding models
//! - **Semantic DHT**: Distributed hash table for semantic indexing
//! - **Router**: Multi-factor semantic routing algorithm
//! - **Node Status**: Node health and load monitoring
//!
//! # Example
//!
//! ```rust,ignore
//! use nexa_net::discovery::{CapabilityRegistry, SemanticRouter};
//! use nexa_net::types::{CapabilitySchema, ServiceMetadata, Did, RouteContext};
//!
//! // Register a capability
//! let mut registry = CapabilityRegistry::new();
//! // registry.register(capability_schema).unwrap();
//!
//! // Discover services by intent
//! let router = SemanticRouter::new(registry);
//! // let routes = router.discover("translate English PDF to Chinese", RouteContext::default()).await?;
//! ```

pub mod capability;
pub mod vectorizer;
pub mod semantic_dht;
pub mod router;
pub mod node_status;
pub mod embedding;

// Re-exports
pub use capability::CapabilityRegistry;
pub use vectorizer::{Vectorizer, VectorizerBuilder, SemanticVector};
pub use semantic_dht::SemanticDHT;
pub use router::SemanticRouter;
pub use node_status::{NodeStatus, NodeStatusManager};

// Embedding re-exports
pub use embedding::{Embedder, EmbeddingConfig, create_embedder};
pub use embedding::mock::MockEmbedder;

#[cfg(feature = "embedding-onnx")]
pub use embedding::onnx::OnnxEmbedder;

// Re-export from types
pub use crate::types::{CapabilitySchema, EndpointDefinition, ServiceMetadata, Route, RouteContext};