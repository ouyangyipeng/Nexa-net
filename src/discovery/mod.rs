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
//! ```rust,no_run
//! use nexa_net::discovery::{CapabilityRegistry, SemanticRouter, RouteContext};
//!
//! // Register a capability
//! let registry = CapabilityRegistry::new();
//! registry.register(capability_schema).await?;
//!
//! // Discover services by intent
//! let router = SemanticRouter::new(registry);
//! let routes = router.discover("translate English PDF to Chinese", RouteContext::default()).await?;
//!
//! for route in routes {
//!     println!("Found: {} (similarity: {})", route.endpoint.name, route.similarity_score);
//! }
//! # Ok::<(), nexa_net::Error>(())
//! ```

pub mod capability;
pub mod vectorizer;
pub mod semantic_dht;
pub mod router;
pub mod node_status;

// Re-exports
pub use capability::CapabilityRegistry;
pub use vectorizer::{Vectorizer, SemanticVector};
pub use semantic_dht::SemanticDHT;
pub use router::SemanticRouter;
pub use node_status::{NodeStatus, NodeStatusManager};

// Re-export from types
pub use crate::types::{CapabilitySchema, EndpointDefinition, ServiceMetadata, Route, RouteContext};