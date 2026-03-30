//! Protocol message definitions
//!
//! This module contains the generated Protobuf message types for Nexa-net.
//! Messages are generated from `proto/*.proto` files during build.

// Note: These will be generated from proto files
// For now, we define placeholder types

pub mod message;
pub mod identity;
pub mod discovery;
pub mod transport;
pub mod economy;

// Re-exports
pub use message::{NexaMessage, MessageHeader, MessageSignature};