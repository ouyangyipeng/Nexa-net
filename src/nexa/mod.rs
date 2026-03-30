//! Nexa Language Integration
//!
//! This module provides integration with the Nexa Agent-Native programming language,
//! including AVM (Agent Virtual Machine) runtime and DAG execution.
//!
//! # Components
//!
//! - **Runtime**: AVM runtime interface
//! - **DAG Executor**: DAG topology execution engine
//! - **Network Bridge**: Bridge between Nexa constructs and Nexa-net network
//!
//! # Nexa Language Mapping
//!
//! | Nexa Construct | Nexa-net Entity |
//! |----------------|-----------------|
//! | `agent` | DID registration |
//! | `tool` | Capability Schema |
//! | `protocol` | RPC interface |
//! | `flow` | Network topology |
//! | `@budget` | State channel budget |

pub mod runtime;
pub mod dag_executor;
pub mod network_bridge;

// Re-exports
pub use runtime::AvmRuntime;
pub use dag_executor::{DagExecutor, DagNode, DagOperator};
pub use network_bridge::NetworkBridge;