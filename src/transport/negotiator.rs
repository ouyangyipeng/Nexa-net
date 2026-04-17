//! Protocol Negotiator
//!
//! Handles SYN-NEXA/ACK-SCHEMA handshake for protocol negotiation.
//!
//! # Handshake Flow
//!
//! ```text
//! Client                                    Server
//!   |                                         |
//!   |------------ SYN-NEXA ----------------->|
//!   |  (protocols, encodings, compressions)  |
//!   |                                         |
//!   |<----------- ACK-SCHEMA ----------------|
//!   |  (selected protocol, schema, cost)     |
//!   |                                         |
//!   |------------ ACCEPT -------------------->|
//!   |  (session_id, ready)                   |
//!   |                                         |
//!   |<----------- ACCEPT (or REJECT) --------|
//!   |                                         |
//! ```

use crate::error::{Error, Result};
use crate::transport::serialization::CompressionAlgorithm;
use crate::types::{Encoding, Protocol};
use std::time::{Duration, Instant};

/// Compression type enumeration (matching Protobuf schema)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompressionType {
    /// Unspecified
    Unspecified = 0,
    #[default]
    /// No compression
    None = 1,
    /// Gzip compression
    Gzip = 2,
    /// LZ4 compression
    Lz4 = 3,
    /// Zstd compression
    Zstd = 4,
}

impl From<CompressionType> for CompressionAlgorithm {
    fn from(ct: CompressionType) -> Self {
        match ct {
            CompressionType::None => CompressionAlgorithm::None,
            CompressionType::Gzip => CompressionAlgorithm::Gzip,
            CompressionType::Lz4 => CompressionAlgorithm::Lz4,
            CompressionType::Zstd => CompressionAlgorithm::Zstd,
            CompressionType::Unspecified => CompressionAlgorithm::None,
        }
    }
}

impl From<CompressionAlgorithm> for CompressionType {
    fn from(ca: CompressionAlgorithm) -> Self {
        match ca {
            CompressionAlgorithm::None => CompressionType::None,
            CompressionAlgorithm::Gzip => CompressionType::Gzip,
            CompressionAlgorithm::Lz4 => CompressionType::Lz4,
            CompressionAlgorithm::Zstd => CompressionType::Zstd,
        }
    }
}

/// Client capabilities
#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    /// Maximum concurrent streams
    pub max_concurrent_streams: u32,
    /// Maximum message size
    pub max_message_size: u64,
    /// Supports streaming
    pub streaming: bool,
    /// Supports bidirectional streaming
    pub bidirectional: bool,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 100,
            max_message_size: 16 * 1024 * 1024, // 16MB
            streaming: true,
            bidirectional: true,
        }
    }
}

/// Server capabilities
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    /// Maximum concurrent streams
    pub max_concurrent_streams: u32,
    /// Maximum message size
    pub max_message_size: u64,
    /// Current load (0.0 - 1.0)
    pub current_load: f32,
    /// Available queue slots
    pub available_queue_slots: u32,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 1000,
            max_message_size: 64 * 1024 * 1024, // 64MB
            current_load: 0.0,
            available_queue_slots: 100,
        }
    }
}

/// Reject reason enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RejectReason {
    #[default]
    /// Unspecified
    Unspecified = 0,
    /// Unsupported protocol
    UnsupportedProtocol = 1,
    /// Insufficient budget
    InsufficientBudget = 2,
    /// Service unavailable
    ServiceUnavailable = 3,
    /// Rate limited
    RateLimited = 4,
    /// Unauthorized
    Unauthorized = 5,
}

/// SYN-NEXA message (client -> server)
#[derive(Debug, Clone)]
pub struct SynNexa {
    /// Intent hash (identifies the service being requested)
    pub intent_hash: String,
    /// Maximum budget for this session
    pub max_budget: u64,
    /// Supported protocols (in preference order)
    pub supported_protocols: Vec<String>,
    /// Supported encodings (in preference order)
    pub supported_encodings: Vec<String>,
    /// Supported compression algorithms
    pub supported_compressions: Vec<CompressionType>,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Timestamp
    pub timestamp: u64,
    /// Signature
    pub signature: Vec<u8>,
}

impl SynNexa {
    /// Create a new SYN-NEXA message
    pub fn new(intent_hash: String, max_budget: u64) -> Self {
        Self {
            intent_hash,
            max_budget,
            supported_protocols: vec!["nexa-rpc-v1".to_string(), "grpc".to_string()],
            supported_encodings: vec!["protobuf".to_string(), "json".to_string()],
            supported_compressions: vec![CompressionType::Lz4, CompressionType::None],
            capabilities: ClientCapabilities::default(),
            timestamp: current_timestamp(),
            signature: Vec::new(),
        }
    }

    /// Set supported protocols
    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.supported_protocols = protocols;
        self
    }

    /// Set supported encodings
    pub fn with_encodings(mut self, encodings: Vec<String>) -> Self {
        self.supported_encodings = encodings;
        self
    }

    /// Set supported compressions
    pub fn with_compressions(mut self, compressions: Vec<CompressionType>) -> Self {
        self.supported_compressions = compressions;
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = signature;
        self
    }
}

/// ACK-SCHEMA message (server -> client)
#[derive(Debug, Clone)]
pub struct AckSchema {
    /// Selected protocol
    pub selected_protocol: String,
    /// Selected encoding
    pub selected_encoding: String,
    /// Selected compression
    pub selected_compression: CompressionType,
    /// Schema hash
    pub schema_hash: String,
    /// Compressed schema (optional)
    pub compressed_schema: Option<Vec<u8>>,
    /// Estimated cost
    pub estimated_cost: u64,
    /// Estimated latency in milliseconds
    pub estimated_latency_ms: u32,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Timestamp
    pub timestamp: u64,
    /// Signature
    pub signature: Vec<u8>,
}

impl AckSchema {
    /// Create a new ACK-SCHEMA message
    pub fn new(protocol: String, encoding: String, compression: CompressionType) -> Self {
        Self {
            selected_protocol: protocol,
            selected_encoding: encoding,
            selected_compression: compression,
            schema_hash: String::new(),
            compressed_schema: None,
            estimated_cost: 0,
            estimated_latency_ms: 100,
            capabilities: ServerCapabilities::default(),
            timestamp: current_timestamp(),
            signature: Vec::new(),
        }
    }

    /// Set schema
    pub fn with_schema(mut self, hash: String, compressed: Option<Vec<u8>>) -> Self {
        self.schema_hash = hash;
        self.compressed_schema = compressed;
        self
    }

    /// Set estimated cost
    pub fn with_cost(mut self, cost: u64) -> Self {
        self.estimated_cost = cost;
        self
    }

    /// Set estimated latency
    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.estimated_latency_ms = latency_ms;
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = signature;
        self
    }
}

/// ACCEPT message (client -> server)
#[derive(Debug, Clone)]
pub struct Accept {
    /// Session ID
    pub session_id: String,
    /// Ready flag
    pub ready: bool,
    /// Error message (if not ready)
    pub error_message: Option<String>,
}

impl Accept {
    /// Create a new ACCEPT message
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            ready: true,
            error_message: None,
        }
    }

    /// Create a rejection ACCEPT
    pub fn reject(session_id: String, error: String) -> Self {
        Self {
            session_id,
            ready: false,
            error_message: Some(error),
        }
    }
}

/// REJECT message (server -> client)
#[derive(Debug, Clone)]
pub struct Reject {
    /// Reject reason
    pub reason: RejectReason,
    /// Detailed message
    pub message: String,
    /// Suggested alternatives
    pub alternatives: Vec<String>,
}

impl Reject {
    /// Create a new REJECT message
    pub fn new(reason: RejectReason, message: String) -> Self {
        Self {
            reason,
            message,
            alternatives: Vec::new(),
        }
    }

    /// Add alternative
    pub fn with_alternative(mut self, alt: String) -> Self {
        self.alternatives.push(alt);
        self
    }
}

/// Negotiated protocol configuration
#[derive(Debug, Clone)]
pub struct NegotiatedProtocol {
    /// Selected protocol
    pub protocol: Protocol,
    /// Selected encoding
    pub encoding: Encoding,
    /// Selected compression
    pub compression: CompressionType,
    /// Schema hash (if applicable)
    pub schema_hash: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Estimated cost
    pub estimated_cost: u64,
    /// Server capabilities
    pub server_capabilities: Option<ServerCapabilities>,
}

/// Negotiation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NegotiationState {
    #[default]
    /// Initial state
    Initial,
    /// SYN-NEXA sent, waiting for ACK-SCHEMA
    SynSent,
    /// ACK-SCHEMA received, sending ACCEPT
    AckReceived,
    /// Negotiation complete
    Complete,
    /// Negotiation failed
    Failed,
}

/// Protocol negotiator (client side)
#[derive(Debug)]
pub struct Negotiator {
    /// Supported protocols (in preference order)
    supported_protocols: Vec<Protocol>,
    /// Supported encodings (in preference order)
    supported_encodings: Vec<Encoding>,
    /// Supported compressions (in preference order)
    supported_compressions: Vec<CompressionType>,
    /// Current state
    state: NegotiationState,
    /// Negotiation result
    result: Option<NegotiatedProtocol>,
    /// Negotiation timeout
    timeout: Duration,
    /// Start time
    start_time: Option<Instant>,
}

impl Negotiator {
    /// Create a new negotiator with default settings
    pub fn new() -> Self {
        Self {
            supported_protocols: vec![Protocol::NexaRpcV1, Protocol::Grpc],
            supported_encodings: vec![Encoding::Lz4, Encoding::Gzip, Encoding::None],
            supported_compressions: vec![
                CompressionType::Lz4,
                CompressionType::Zstd,
                CompressionType::None,
            ],
            state: NegotiationState::Initial,
            result: None,
            timeout: Duration::from_secs(30),
            start_time: None,
        }
    }

    /// Create a negotiator with custom settings
    pub fn with_settings(
        protocols: Vec<Protocol>,
        encodings: Vec<Encoding>,
        compressions: Vec<CompressionType>,
    ) -> Self {
        Self {
            supported_protocols: protocols,
            supported_encodings: encodings,
            supported_compressions: compressions,
            state: NegotiationState::Initial,
            result: None,
            timeout: Duration::from_secs(30),
            start_time: None,
        }
    }

    /// Set negotiation timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get current state
    pub fn state(&self) -> NegotiationState {
        self.state
    }

    /// Get negotiation result
    pub fn result(&self) -> Option<&NegotiatedProtocol> {
        self.result.as_ref()
    }

    /// Create SYN-NEXA message
    pub fn create_syn(&self, intent_hash: String, max_budget: u64) -> SynNexa {
        SynNexa::new(intent_hash, max_budget)
            .with_protocols(self.protocol_strings())
            .with_encodings(self.encoding_strings())
            .with_compressions(self.supported_compressions.clone())
    }

    /// Process ACK-SCHEMA response
    pub fn process_ack(&mut self, ack: AckSchema) -> Result<()> {
        if self.state != NegotiationState::SynSent {
            return Err(Error::Protocol("Invalid negotiation state".to_string()));
        }

        // Verify selected protocol is supported
        let protocol = self.parse_protocol(&ack.selected_protocol).ok_or_else(|| {
            Error::ProtocolNegotiation(format!("Unsupported protocol: {}", ack.selected_protocol))
        })?;

        // Verify selected encoding is supported
        let encoding = self.parse_encoding(&ack.selected_encoding).ok_or_else(|| {
            Error::ProtocolNegotiation(format!("Unsupported encoding: {}", ack.selected_encoding))
        })?;

        // Verify selected compression is supported
        if !self
            .supported_compressions
            .contains(&ack.selected_compression)
        {
            return Err(Error::ProtocolNegotiation(format!(
                "Unsupported compression: {:?}",
                ack.selected_compression
            )));
        }

        self.result = Some(NegotiatedProtocol {
            protocol,
            encoding,
            compression: ack.selected_compression,
            schema_hash: if ack.schema_hash.is_empty() {
                None
            } else {
                Some(ack.schema_hash)
            },
            session_id: None,
            estimated_cost: ack.estimated_cost,
            server_capabilities: Some(ack.capabilities),
        });

        self.state = NegotiationState::AckReceived;
        Ok(())
    }

    /// Process REJECT response
    pub fn process_reject(&mut self, reject: Reject) -> Result<()> {
        self.state = NegotiationState::Failed;
        Err(Error::ProtocolNegotiation(format!(
            "Negotiation rejected: {:?} - {}",
            reject.reason, reject.message
        )))
    }

    /// Complete negotiation with ACCEPT
    pub fn complete(&mut self, session_id: String) -> Result<NegotiatedProtocol> {
        if self.state != NegotiationState::AckReceived {
            return Err(Error::Protocol("Invalid negotiation state".to_string()));
        }

        if let Some(ref mut result) = self.result {
            result.session_id = Some(session_id);
        }

        self.state = NegotiationState::Complete;
        self.result
            .clone()
            .ok_or_else(|| Error::Protocol("No negotiation result".to_string()))
    }

    /// Start negotiation (internal state update)
    pub fn start(&mut self) {
        self.state = NegotiationState::SynSent;
        self.start_time = Some(Instant::now());
    }

    /// Check if negotiation has timed out
    pub fn is_timed_out(&self) -> bool {
        if let Some(start) = self.start_time {
            return start.elapsed() > self.timeout;
        }
        false
    }

    /// Negotiate protocol with remote peer (simple interface)
    pub async fn negotiate(&self, remote_protocols: &[&str]) -> Result<NegotiatedProtocol> {
        // Find first matching protocol
        let protocol = remote_protocols
            .iter()
            .find_map(|p| self.parse_protocol(p))
            .ok_or_else(|| Error::ProtocolNegotiation("No matching protocol".to_string()))?;

        Ok(NegotiatedProtocol {
            protocol,
            encoding: Encoding::Lz4,
            compression: CompressionType::Lz4,
            schema_hash: None,
            session_id: None,
            estimated_cost: 0,
            server_capabilities: None,
        })
    }

    /// Get protocol strings for SYN message
    fn protocol_strings(&self) -> Vec<String> {
        self.supported_protocols
            .iter()
            .map(|p| p.to_string())
            .collect()
    }

    /// Get encoding strings for SYN message
    fn encoding_strings(&self) -> Vec<String> {
        self.supported_encodings
            .iter()
            .map(|e| e.to_string())
            .collect()
    }

    /// Parse protocol string
    fn parse_protocol(&self, s: &str) -> Option<Protocol> {
        match s.to_lowercase().as_str() {
            "nexa-rpc-v1" | "nexa_rpc_v1" => Some(Protocol::NexaRpcV1),
            "grpc" => Some(Protocol::Grpc),
            "flatbuffers" => Some(Protocol::FlatBuffers),
            _ => None,
        }
    }

    /// Parse encoding string
    fn parse_encoding(&self, s: &str) -> Option<Encoding> {
        match s.to_lowercase().as_str() {
            "lz4" => Some(Encoding::Lz4),
            "gzip" => Some(Encoding::Gzip),
            "none" | "raw" => Some(Encoding::None),
            _ => None,
        }
    }
}

impl Default for Negotiator {
    fn default() -> Self {
        Self::new()
    }
}

/// Server-side negotiator
#[derive(Debug)]
pub struct ServerNegotiator {
    /// Supported protocols
    supported_protocols: Vec<Protocol>,
    /// Supported encodings
    supported_encodings: Vec<Encoding>,
    /// Supported compressions
    supported_compressions: Vec<CompressionType>,
    /// Server capabilities
    capabilities: ServerCapabilities,
}

impl ServerNegotiator {
    /// Create a new server negotiator
    pub fn new() -> Self {
        Self {
            supported_protocols: vec![Protocol::NexaRpcV1, Protocol::Grpc],
            supported_encodings: vec![Encoding::Lz4, Encoding::Gzip, Encoding::None],
            supported_compressions: vec![
                CompressionType::Lz4,
                CompressionType::Zstd,
                CompressionType::None,
            ],
            capabilities: ServerCapabilities::default(),
        }
    }

    /// Set server capabilities
    pub fn with_capabilities(mut self, caps: ServerCapabilities) -> Self {
        self.capabilities = caps;
        self
    }

    /// Process SYN-NEXA and create response
    pub fn process_syn(&self, syn: &SynNexa) -> Result<AckSchema> {
        // Find matching protocol
        let protocol = self
            .select_protocol(&syn.supported_protocols)
            .ok_or_else(|| Error::ProtocolNegotiation("No matching protocol".to_string()))?;

        // Find matching encoding
        let encoding = self
            .select_encoding(&syn.supported_encodings)
            .ok_or_else(|| Error::ProtocolNegotiation("No matching encoding".to_string()))?;

        // Find matching compression
        let compression = self
            .select_compression(&syn.supported_compressions)
            .ok_or_else(|| Error::ProtocolNegotiation("No matching compression".to_string()))?;

        // Check budget
        let estimated_cost = self.estimate_cost(&syn.intent_hash);
        if estimated_cost > syn.max_budget {
            return Err(Error::ProtocolNegotiation(format!(
                "Estimated cost {} exceeds budget {}",
                estimated_cost, syn.max_budget
            )));
        }

        // Create ACK-SCHEMA
        Ok(AckSchema::new(protocol, encoding, compression)
            .with_cost(estimated_cost)
            .with_latency(self.estimate_latency()))
    }

    /// Process SYN-NEXA and create response (with Reject details)
    pub fn process_syn_with_reject(&self, syn: &SynNexa) -> std::result::Result<AckSchema, Reject> {
        // Find matching protocol
        let protocol = self
            .select_protocol(&syn.supported_protocols)
            .ok_or_else(|| {
                Reject::new(
                    RejectReason::UnsupportedProtocol,
                    "No matching protocol".to_string(),
                )
            })?;

        // Find matching encoding
        let encoding = self
            .select_encoding(&syn.supported_encodings)
            .ok_or_else(|| {
                Reject::new(
                    RejectReason::UnsupportedProtocol,
                    "No matching encoding".to_string(),
                )
            })?;

        // Find matching compression
        let compression = self
            .select_compression(&syn.supported_compressions)
            .ok_or_else(|| {
                Reject::new(
                    RejectReason::UnsupportedProtocol,
                    "No matching compression".to_string(),
                )
            })?;

        // Check budget
        let estimated_cost = self.estimate_cost(&syn.intent_hash);
        if estimated_cost > syn.max_budget {
            return Err(Reject::new(
                RejectReason::InsufficientBudget,
                format!(
                    "Estimated cost {} exceeds budget {}",
                    estimated_cost, syn.max_budget
                ),
            ));
        }

        // Create ACK-SCHEMA
        Ok(AckSchema::new(protocol, encoding, compression)
            .with_cost(estimated_cost)
            .with_latency(self.estimate_latency()))
    }

    /// Select best matching protocol
    fn select_protocol(&self, client_protocols: &[String]) -> Option<String> {
        for client_proto in client_protocols {
            for server_proto in &self.supported_protocols {
                if server_proto.to_string().to_lowercase() == client_proto.to_lowercase() {
                    return Some(server_proto.to_string());
                }
            }
        }
        None
    }

    /// Select best matching encoding
    fn select_encoding(&self, client_encodings: &[String]) -> Option<String> {
        for client_enc in client_encodings {
            for server_enc in &self.supported_encodings {
                if server_enc.to_string().to_lowercase() == client_enc.to_lowercase() {
                    return Some(server_enc.to_string());
                }
            }
        }
        None
    }

    /// Select best matching compression
    fn select_compression(
        &self,
        client_compressions: &[CompressionType],
    ) -> Option<CompressionType> {
        for client_comp in client_compressions {
            if self.supported_compressions.contains(client_comp) {
                return Some(*client_comp);
            }
        }
        None
    }

    /// Estimate cost for an intent
    fn estimate_cost(&self, _intent_hash: &str) -> u64 {
        // NOTE: Placeholder cost estimation — actual value from capability metadata
        10
    }

    /// Estimate latency
    fn estimate_latency(&self) -> u32 {
        // NOTE: Placeholder latency estimation — actual value from network monitoring
        50
    }
}

impl Default for ServerNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiator_creation() {
        let negotiator = Negotiator::new();
        assert!(!negotiator.supported_protocols.is_empty());
        assert_eq!(negotiator.state(), NegotiationState::Initial);
    }

    #[test]
    fn test_syn_nexa_creation() {
        let syn = SynNexa::new("intent-hash-123".to_string(), 1000);
        assert_eq!(syn.intent_hash, "intent-hash-123");
        assert_eq!(syn.max_budget, 1000);
        assert!(!syn.supported_protocols.is_empty());
    }

    #[test]
    fn test_ack_schema_creation() {
        let ack = AckSchema::new(
            "nexa-rpc-v1".to_string(),
            "protobuf".to_string(),
            CompressionType::Lz4,
        );
        assert_eq!(ack.selected_protocol, "nexa-rpc-v1");
        assert_eq!(ack.selected_compression, CompressionType::Lz4);
    }

    #[test]
    fn test_accept_creation() {
        let accept = Accept::new("session-123".to_string());
        assert!(accept.ready);
        assert!(accept.error_message.is_none());

        let reject = Accept::reject("session-123".to_string(), "Error".to_string());
        assert!(!reject.ready);
        assert!(reject.error_message.is_some());
    }

    #[test]
    fn test_server_negotiator() {
        let server = ServerNegotiator::new();
        // Create SYN with matching encodings
        let syn = SynNexa::new("intent-hash".to_string(), 1000).with_encodings(vec![
            "lz4".to_string(),
            "gzip".to_string(),
            "none".to_string(),
        ]);

        let result = server.process_syn(&syn);
        assert!(result.is_ok());

        let ack = result.unwrap();
        assert!(!ack.selected_protocol.is_empty());
    }

    #[test]
    fn test_server_negotiator_with_reject() {
        let server = ServerNegotiator::new();
        // Create SYN with matching encodings
        let syn = SynNexa::new("intent-hash".to_string(), 1000).with_encodings(vec![
            "lz4".to_string(),
            "gzip".to_string(),
            "none".to_string(),
        ]);

        let result = server.process_syn_with_reject(&syn);
        assert!(result.is_ok());

        let ack = result.unwrap();
        assert!(!ack.selected_protocol.is_empty());
    }

    #[test]
    fn test_compression_type_conversion() {
        let ct = CompressionType::Lz4;
        let ca: CompressionAlgorithm = ct.into();
        assert_eq!(ca, CompressionAlgorithm::Lz4);

        let ca2 = CompressionAlgorithm::Gzip;
        let ct2: CompressionType = ca2.into();
        assert_eq!(ct2, CompressionType::Gzip);
    }

    #[test]
    fn test_negotiation_flow() {
        let mut client = Negotiator::new();

        // Create SYN
        let syn = client.create_syn("intent-hash".to_string(), 1000);
        assert!(!syn.supported_protocols.is_empty());

        // Start negotiation
        client.start();
        assert_eq!(client.state(), NegotiationState::SynSent);

        // Process ACK
        let ack = AckSchema::new(
            "nexa-rpc-v1".to_string(),
            "lz4".to_string(),
            CompressionType::Lz4,
        );
        client.process_ack(ack).unwrap();
        assert_eq!(client.state(), NegotiationState::AckReceived);

        // Complete
        let result = client.complete("session-123".to_string()).unwrap();
        assert_eq!(client.state(), NegotiationState::Complete);
        assert_eq!(result.session_id, Some("session-123".to_string()));
    }
}
