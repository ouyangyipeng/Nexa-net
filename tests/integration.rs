//! Integration Tests for Nexa-net
//!
//! This module contains integration tests that simulate multi-agent
//! communication scenarios across all four layers:
//! - Layer 1 (Identity): DID, VC, key management
//! - Layer 2 (Discovery): Capability registration, semantic routing
//! - Layer 3 (Transport): Binary RPC protocol
//! - Layer 4 (Economy): State channels, receipts, settlement

#[path = "common/mod.rs"]
mod common;

mod channel_test;
mod discovery_test;
