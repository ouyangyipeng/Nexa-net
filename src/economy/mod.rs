//! Layer 4: M2M Economy Layer
//!
//! This module implements micro-transactions and state channels
//! for Nexa-net agents.
//!
//! # Components
//!
//! - **Token**: Nexa-Token definition and balance management
//! - **Channel**: State channel management (open, update, close)
//! - **Receipt**: Micro-receipt generation and verification
//! - **Budget**: Budget control and resource guardrails
//! - **Settlement**: Settlement and dispute resolution
//!
//! # Example
//!
//! ```rust,ignore
//! use nexa_net::economy::{ChannelManager, BudgetController, MicroReceipt};
//! use nexa_net::types::Did;
//!
//! // Open a state channel
//! let mut channel_manager = ChannelManager::new();
//! let party_a = Did::new("did:nexa:alice");
//! let party_b = Did::new("did:nexa:bob");
//! let channel = channel_manager.open(party_a, party_b, 1000, 500).unwrap();
//!
//! // Create a receipt for a call
//! let receipt = MicroReceipt::new("call-1", payer_did, payee_did, 25);
//! ```

pub mod budget;
pub mod channel;
pub mod receipt;
pub mod settlement;
pub mod token;

// Re-exports
pub use budget::{BudgetController, BudgetLimit, BudgetStatus};
pub use channel::{Channel, ChannelConfig, ChannelId, ChannelManager};
pub use receipt::{MicroReceipt, ReceiptChain, ReceiptVerifier};
pub use settlement::{Dispute, Settlement, SettlementEngine};
pub use token::{TokenAmount, TokenBalance, TokenEngine};

// Re-export from types
pub use crate::types::ChannelState;
