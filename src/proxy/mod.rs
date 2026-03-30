//! Nexa-Proxy: Local Sidecar Proxy
//!
//! This module implements the Nexa-Proxy daemon that runs alongside each Agent
//! to handle network communication, routing, encryption, and settlement.
//!
//! # Components
//!
//! - **Server**: Local REST/gRPC API server
//! - **Client**: Network client for outbound calls
//! - **Config**: Configuration management
//!
//! # Example
//!
//! ```rust,no_run
//! use nexa_net::proxy::{ProxyServer, ProxyConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ProxyConfig::default();
//!     let server = ProxyServer::new(config);
//!     
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

pub mod server;
pub mod client;
pub mod config;

// Re-exports
pub use server::ProxyServer;
pub use client::ProxyClient;
pub use config::ProxyConfig;