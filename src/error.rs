//! Error types for Nexa-net

use thiserror::Error;

/// Result type alias for Nexa-net operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for Nexa-net
#[derive(Error, Debug)]
pub enum Error {
    // Configuration Errors
    #[error("Configuration error: {0}")]
    Config(String),

    // Identity Layer Errors
    #[error("DID generation failed: {0}")]
    DidGeneration(String),

    #[error("DID parsing failed: {0}")]
    DidParsing(String),

    #[error("Invalid DID format: {0}")]
    InvalidDidFormat(String),

    #[error("Key generation failed: {0}")]
    KeyGeneration(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerification(String),

    #[error("Credential verification failed: {0}")]
    CredentialVerification(String),

    #[error("mTLS handshake failed: {0}")]
    MtlsHandshake(String),

    // Discovery Layer Errors
    #[error("Service not found for intent: {0}")]
    ServiceNotFound(String),

    #[error("Capability registration failed: {0}")]
    CapabilityRegistration(String),

    #[error("DHT operation failed: {0}")]
    DhtOperation(String),

    #[error("Vectorization failed: {0}")]
    Vectorization(String),

    #[error("No matching service found (similarity: {0}, threshold: {1})")]
    NoMatchingService(f32, f32),

    // Transport Layer Errors
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Protocol negotiation failed: {0}")]
    ProtocolNegotiation(String),

    #[error("RPC call failed: {0}")]
    RpcCall(String),

    #[error("Serialization failed: {0}")]
    Serialization(String),

    #[error("Deserialization failed: {0}")]
    Deserialization(String),

    #[error("Connection timeout after {0}ms")]
    ConnectionTimeout(u64),

    #[error("Stream closed unexpectedly")]
    StreamClosed,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Timeout after {0}ms")]
    Timeout(u32),

    #[error("Frame error: {0}")]
    Frame(String),

    #[error("Flow control error: {0}")]
    FlowControl(String),

    // Economy Layer Errors
    #[error("Insufficient balance: required {0}, available {1}")]
    InsufficientBalance(u64, u64),

    #[error("Channel operation failed: {0}")]
    ChannelOperation(String),

    #[error("Budget exceeded: limit {0}, spent {1}")]
    BudgetExceeded(u64, u64),

    #[error("Receipt verification failed: {0}")]
    ReceiptVerification(String),

    #[error("Settlement failed: {0}")]
    Settlement(String),

    // Protocol Errors
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),

    #[error("Unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(String),

    #[error("Message validation failed: {0}")]
    MessageValidation(String),

    // Configuration Errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Invalid configuration file: {0}")]
    InvalidConfigFile(String),

    // I/O Errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    // Network Errors
    #[error("Network error: {0}")]
    Network(String),

    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),

    // Internal Errors
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    // External Errors
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("Serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl From<prost::DecodeError> for Error {
    fn from(err: prost::DecodeError) -> Self {
        Error::Deserialization(err.to_string())
    }
}

impl From<prost::EncodeError> for Error {
    fn from(err: prost::EncodeError) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        Error::SignatureVerification(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::ServiceNotFound("translate text".to_string());
        assert!(err.to_string().contains("translate text"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }
}
