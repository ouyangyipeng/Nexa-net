//! Convenience constructors and helpers for protocol messages
//!
//! Proto-generated types use `Option<T>` for message fields.
//! This module provides helper functions that handle Option wrapping
//! and provide convenient constructors.

use crate::protocol::{MessageHeader, MessageSignature, NexaMessage};

/// Create a new NexaMessage with standard defaults
pub fn create_message(
    message_type: &str,
    sender_did: &str,
    body: Vec<u8>,
    ttl: u32,
) -> NexaMessage {
    let now = chrono::Utc::now();
    let timestamp = prost_types::Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    };

    NexaMessage {
        header: Some(MessageHeader {
            protocol_version: "v1".to_string(),
            message_type: message_type.to_string(),
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: String::new(),
            sender_did: sender_did.to_string(),
            receiver_did: String::new(),
            timestamp: Some(timestamp),
            ttl,
            metadata: Default::default(),
        }),
        body,
        signature: Some(MessageSignature {
            algorithm: "Ed25519".to_string(),
            signature: Vec::new(),
            signed_at: None,
            verification_key_ref: String::new(),
        }),
    }
}

/// Create a response message correlated to a request
pub fn create_response(request: &NexaMessage, body: Vec<u8>) -> NexaMessage {
    let req_header = request.header.as_ref().expect("request must have a header");

    let now = chrono::Utc::now();
    let timestamp = prost_types::Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    };

    NexaMessage {
        header: Some(MessageHeader {
            protocol_version: req_header.protocol_version.clone(),
            message_type: format!("{}_RESPONSE", req_header.message_type),
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: req_header.message_id.clone(),
            sender_did: req_header.receiver_did.clone(),
            receiver_did: req_header.sender_did.clone(),
            timestamp: Some(timestamp),
            ttl: req_header.ttl,
            metadata: Default::default(),
        }),
        body,
        signature: Some(MessageSignature {
            algorithm: "Ed25519".to_string(),
            signature: Vec::new(),
            signed_at: None,
            verification_key_ref: String::new(),
        }),
    }
}

/// Validate that a message has required fields populated
pub fn validate_message(msg: &NexaMessage) -> Result<(), String> {
    let header = msg.header.as_ref();
    if header.is_none() {
        return Err("header is required".to_string());
    }
    let h = header.unwrap();
    if h.protocol_version.is_empty() {
        return Err("protocol_version is required".to_string());
    }
    if h.message_type.is_empty() {
        return Err("message_type is required".to_string());
    }
    if h.message_id.is_empty() {
        return Err("message_id is required".to_string());
    }
    if h.sender_did.is_empty() {
        return Err("sender_did is required".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_message() {
        let msg = create_message("DID_RESOLVE", "did:nexa:test", vec![1, 2, 3], 300);
        let h = msg.header.unwrap();
        assert_eq!(h.protocol_version, "v1");
        assert_eq!(h.message_type, "DID_RESOLVE");
        assert!(!h.message_id.is_empty());
        assert_eq!(h.sender_did, "did:nexa:test");
        assert_eq!(h.ttl, 300);
    }

    #[test]
    fn test_create_response() {
        let request = create_message("ROUTE_QUERY", "did:nexa:caller", vec![], 300);
        let response = create_response(&request, vec![4, 5, 6]);
        let req_h = request.header.unwrap();
        let resp_h = response.header.unwrap();
        assert_eq!(resp_h.correlation_id, req_h.message_id);
        assert_eq!(resp_h.receiver_did, "did:nexa:caller");
    }

    #[test]
    fn test_validate_message() {
        let msg = create_message("DID_RESOLVE", "did:nexa:test", vec![], 300);
        assert!(validate_message(&msg).is_ok());

        let empty_msg = NexaMessage::default();
        assert!(validate_message(&empty_msg).is_err());
    }
}
