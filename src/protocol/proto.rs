//! Generated Protobuf sub-package types
//!
//! These types belong to proto sub-packages (identity, discovery, transport, economy).
//! They reference common types via `super::`, which resolves to the parent module
//! (src/protocol/mod.rs) where the common types are included.

// Layer 1: Identity protocol types
include!(concat!(env!("OUT_DIR"), "/nexa.protocol.identity.rs"));

// Layer 2: Discovery protocol types
include!(concat!(env!("OUT_DIR"), "/nexa.protocol.discovery.rs"));

// Layer 3: Transport protocol types
include!(concat!(env!("OUT_DIR"), "/nexa.protocol.transport.rs"));

// Layer 4: Economy protocol types
include!(concat!(env!("OUT_DIR"), "/nexa.protocol.economy.rs"));
