//! Layer 2: Semantic Discovery & Capability Routing Layer
//!
//! This module implements semantic service discovery and capability-based routing
//! for Nexa-net agents.
//!
//! # Components
//!
//! - **HNSW Index**: Hierarchical Navigable Small World graph for O(log n) vector search
//! - **Kademlia DHT**: Distributed hash table with k-bucket routing for node discovery
//! - **Semantic DHT**: Combined HNSW + Kademlia for distributed semantic search
//! - **Capability**: Service capability schema definition and registration
//! - **Vectorizer**: Semantic vectorization using embedding models
//! - **Router**: Multi-factor semantic routing algorithm
//! - **Node Status**: Node health and load monitoring
//!
//! # Architecture
//!
//! The Semantic DHT combines two key algorithms:
//! 1. **HNSW** (local): Fast approximate nearest neighbor search for semantic matching
//! 2. **Kademlia** (distributed): DHT routing for node discovery and vector replication
//!
//! This enables sub-linear search complexity across the entire network:
//! - Local search: O(log n) via HNSW graph traversal
//! - Network search: O(log N) via Kademlia-style iterative lookup

pub mod capability;
pub mod embedding;
pub mod node_status;
pub mod router;
pub mod semantic_dht;
pub mod vectorizer;

// Re-exports
pub use capability::CapabilityRegistry;
pub use node_status::{NodeStatus, NodeStatusManager};
pub use router::SemanticRouter;
pub use semantic_dht::{DhtNodeInfo, HnswConfig, HnswIndex, KademliaRoutingTable, SemanticDHT};
pub use vectorizer::{SemanticVector, Vectorizer, VectorizerBuilder};

// Embedding re-exports
pub use embedding::mock::MockEmbedder;
pub use embedding::{create_embedder, Embedder, EmbeddingConfig};

#[cfg(feature = "embedding-onnx")]
pub use embedding::onnx::OnnxEmbedder;

// Re-export from types
pub use crate::types::{
    CapabilitySchema, EndpointDefinition, Route, RouteContext, ServiceMetadata,
};
