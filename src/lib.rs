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
//! ```rust,no_run
//! use nexa_net::{NexaClient, CallRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = NexaClient::new("http://127.0.0.1:7070").await?;
//!     
//!     let request = CallRequest {
//!         intent: "translate English text to Chinese".to_string(),
//!         data: vec![],
//!         max_budget: 50,
//!         timeout_ms: 30000,
//!         ..Default::default()
//!     };
//!     
//!     let response = client.call(request).await?;
//!     println!("Result: {:?}", response.result);
//!     
//!     Ok(())
//! }
//! ```

pub mod identity;
pub mod discovery;
pub mod transport;
pub mod economy;
pub mod protocol;
pub mod proxy;
pub mod nexa;
pub mod api;

pub mod types;
pub mod error;

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