//! Protocol message definitions

use serde::{Deserialize, Serialize};

/// Nexa message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexaMessage {
    /// Message header
    pub header: MessageHeader,
    /// Message body (serialized)
    pub body: Vec<u8>,
    /// Message signature
    pub signature: MessageSignature,
}

/// Message header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Protocol version
    pub protocol_version: String,
    /// Message type
    pub message_type: String,
    /// Message ID
    pub message_id: String,
    /// Correlation ID
    pub correlation_id: Option<String>,
    /// Sender DID
    pub sender_did: String,
    /// Receiver DID
    pub receiver_did: Option<String>,
    /// Timestamp
    pub timestamp: i64,
    /// TTL in seconds
    pub ttl: u32,
}

/// Message signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSignature {
    /// Signature algorithm
    pub algorithm: String,
    /// Signature bytes
    pub signature: Vec<u8>,
}

impl NexaMessage {
    /// Create a new message
    pub fn new(message_type: &str, sender_did: &str, body: Vec<u8>) -> Self {
        Self {
            header: MessageHeader {
                protocol_version: "v1".to_string(),
                message_type: message_type.to_string(),
                message_id: uuid::Uuid::new_v4().to_string(),
                correlation_id: None,
                sender_did: sender_did.to_string(),
                receiver_did: None,
                timestamp: chrono::Utc::now().timestamp(),
                ttl: 300,
            },
            body,
            signature: MessageSignature {
                algorithm: "Ed25519".to_string(),
                signature: vec![],
            },
        }
    }
}