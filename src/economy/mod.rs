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
//! ```rust,no_run
//! use nexa_net::economy::{ChannelManager, BudgetController, MicroReceipt};
//!
//! // Open a state channel
//! let channel_manager = ChannelManager::new();
//! let channel = channel_manager.open(peer_did, 1000).await?;
//!
//! // Create a receipt for a call
//! let receipt = MicroReceipt::new(call_id, payer_did, payee_did, 25);
//!
//! // Settle the channel
//! channel_manager.settle(channel.id()).await?;
//! # Ok::<(), nexa_net::Error>(())
//! ```

pub mod token;
pub mod channel;
pub mod receipt;
pub mod budget;
pub mod settlement;

// Re-exports
pub use token::{TokenEngine, TokenBalance, TokenAmount};
pub use channel::{ChannelManager, Channel, ChannelId};
pub use receipt::{MicroReceipt, ReceiptSigner, ReceiptVerifier};
pub use budget::{BudgetController, BudgetLimit, BudgetStatus};
pub use settlement::{SettlementEngine, Settlement, Dispute};

// Re-export from types
pub use crate::types::ChannelState;