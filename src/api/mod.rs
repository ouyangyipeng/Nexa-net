//! API Layer: REST, gRPC, and SDK interfaces
//!
//! This module provides the public API interfaces for Nexa-net.
//!
//! # Components
//!
//! - **REST**: HTTP REST API server
//! - **gRPC**: gRPC service definitions
//! - **SDK**: High-level SDK for developers

pub mod grpc;
pub mod rest;
pub mod sdk;

// Re-exports
pub use grpc::GrpcServer;
pub use rest::RestServer;
pub use sdk::{NexaClient, NexaClientBuilder};
