//! Nexa-net: Decentralized M2M communication infrastructure for autonomous agents
//!
//! Nexa-net provides a four-layer architecture for secure, efficient machine-to-machine
//! communication between autonomous agents:
//!
//! - **Layer 1 (Identity)**: Decentralized identity (DID) and zero-trust authentication
//! - **Layer 2 (Discovery)**: Semantic service discovery and capability routing
//! - **Layer 3 (Transport)**: Binary RPC protocol with streaming support
//! - **Layer 4 (Economy)**: Micro-transactions and state channels
//!
//! # Example
//!
//! ```rust,ignore
//! use nexa_net::api::sdk::NexaClientBuilder;
//! use nexa_net::types::CallRequest;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = NexaClientBuilder::new()
//!         .endpoint("http://127.0.0.1:7070")
//!         .timeout_ms(30000)
//!         .budget(50)
//!         .build();
//!
//!     let request = CallRequest {
//!         intent: "translate English text to Chinese".to_string(),
//!         data: vec![],
//!         max_budget: 50,
//!         timeout_ms: 30000,
//!         ..Default::default()
//!     };
//!
//!     // let response = client.call(request).await?;
//!     // println!("Result: {:?}", response.result);
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod discovery;
pub mod economy;
pub mod identity;
pub mod nexa;
pub mod protocol;
pub mod proxy;
pub mod security;
pub mod storage;
pub mod transport;

pub mod error;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use types::*;

/// Nexa-net version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Nexa-net protocol version
pub const PROTOCOL_VERSION: &str = "v1";

/// Default local API port
pub const DEFAULT_API_PORT: u16 = 7070;

/// Default gRPC port
pub const DEFAULT_GRPC_PORT: u16 = 7071;
