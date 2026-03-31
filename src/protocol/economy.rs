//! Economy protocol messages

use serde::{Deserialize, Serialize};

/// Open channel request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenChannelRequest {
    /// Peer DID
    pub peer_did: String,
    /// Deposit amount
    pub deposit: u64,
}

/// Open channel response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenChannelResponse {
    /// Channel ID
    pub channel_id: String,
    /// Success
    pub success: bool,
}

/// Close channel request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseChannelRequest {
    /// Channel ID
    pub channel_id: String,
}

/// Payment receipt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceiptMessage {
    /// Receipt ID
    pub receipt_id: String,
    /// Call ID
    pub call_id: String,
    /// Amount
    pub amount: u64,
    /// Signature
    pub signature: Vec<u8>,
}
