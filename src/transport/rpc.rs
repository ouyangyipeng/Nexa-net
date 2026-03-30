//! RPC Engine
//!
//! Streaming RPC with multiplexing support.
//!
//! # RPC Models
//!
//! - **Unary**: Single request, single response
//! - **Server Streaming**: Single request, multiple responses
//! - **Client Streaming**: Multiple requests, single response
//! - **Bidirectional Streaming**: Multiple requests, multiple responses

use crate::error::{Error, Result};
use crate::transport::frame::{Frame, FrameFlags, FrameType};
use crate::transport::stream::{FlowController, StreamId, StreamManager};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};

/// RPC status enumeration (matching Protobuf schema)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcStatus {
    /// Success
    Success = 0,
    /// General error
    Error = 1,
    /// Timeout
    Timeout = 2,
    /// Cancelled by client
    Cancelled = 3,
    /// Insufficient budget
    InsufficientBudget = 4,
    /// Rate limited
    RateLimited = 5,
    /// Internal error
    InternalError = 6,
}

impl Default for RpcStatus {
    fn default() -> Self {
        RpcStatus::Success
    }
}

/// Error type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Transient error, can retry
    Transient = 0,
    /// Permanent error, cannot retry
    Permanent = 1,
    /// Client error
    Client = 2,
    /// Server error
    Server = 3,
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Whether retry is allowed
    pub retryable: bool,
    /// Maximum retry count
    pub max_retries: u32,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u32,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u32,
    /// Delay multiplier
    pub delay_multiplier: f32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            retryable: true,
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            delay_multiplier: 2.0,
        }
    }
}

/// Error detail structure
#[derive(Debug, Clone)]
pub struct ErrorDetail {
    /// Error code
    pub error_code: String,
    /// Error type
    pub error_type: ErrorType,
    /// Retry policy
    pub retry_policy: RetryPolicy,
}

/// RPC header for requests
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RpcHeader {
    /// Method name
    pub method: String,
    /// Call ID
    pub call_id: u64,
    /// Caller DID
    pub caller_did: String,
    /// Target endpoint ID
    pub endpoint_id: String,
    /// Budget
    pub budget: u32,
    /// Timeout in milliseconds
    pub timeout_ms: u32,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp
    pub timestamp: u64,
    /// Signature
    pub signature: Vec<u8>,
}

impl RpcHeader {
    /// Create a new RPC header
    pub fn new(method: String, call_id: u64, caller_did: String) -> Self {
        Self {
            method,
            call_id,
            caller_did,
            endpoint_id: String::new(),
            budget: 0,
            timeout_ms: 30000,
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            signature: Vec::new(),
        }
    }

    /// Set endpoint
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint_id = endpoint;
        self
    }

    /// Set budget
    pub fn with_budget(mut self, budget: u32) -> Self {
        self.budget = budget;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = signature;
        self
    }
}

/// RPC response header
#[derive(Debug, Clone)]
pub struct RpcResponseHeader {
    /// Call ID (matching request)
    pub call_id: u64,
    /// Status
    pub status: RpcStatus,
    /// Actual cost
    pub actual_cost: u32,
    /// Processing time in milliseconds
    pub processing_time_ms: u32,
    /// Error message
    pub error_message: Option<String>,
    /// Error detail
    pub error_detail: Option<ErrorDetail>,
    /// Timestamp
    pub timestamp: u64,
    /// Signature
    pub signature: Vec<u8>,
}

impl RpcResponseHeader {
    /// Create a success response
    pub fn success(call_id: u64, cost: u32, processing_time_ms: u32) -> Self {
        Self {
            call_id,
            status: RpcStatus::Success,
            actual_cost: cost,
            processing_time_ms,
            error_message: None,
            error_detail: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            signature: Vec::new(),
        }
    }

    /// Create an error response
    pub fn error(call_id: u64, status: RpcStatus, message: String) -> Self {
        Self {
            call_id,
            status,
            actual_cost: 0,
            processing_time_ms: 0,
            error_message: Some(message),
            error_detail: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            signature: Vec::new(),
        }
    }

    /// Set signature
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = signature;
        self
    }
}

/// Data type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// Binary data
    Binary = 0,
    /// Text data
    Text = 1,
    /// JSON data
    Json = 2,
    /// Protobuf data
    Protobuf = 3,
    /// Image data
    Image = 4,
    /// Audio data
    Audio = 5,
    /// Video data
    Video = 6,
}

impl Default for DataType {
    fn default() -> Self {
        DataType::Binary
    }
}

/// Data frame payload
#[derive(Debug, Clone)]
pub struct DataFrame {
    /// Stream ID
    pub stream_id: StreamId,
    /// Sequence number
    pub sequence: u64,
    /// Whether compressed
    pub compressed: bool,
    /// Data bytes
    pub data: Vec<u8>,
    /// Data type
    pub data_type: DataType,
}

impl DataFrame {
    /// Create a new data frame
    pub fn new(stream_id: StreamId, sequence: u64, data: Vec<u8>) -> Self {
        Self {
            stream_id,
            sequence,
            compressed: false,
            data,
            data_type: DataType::Binary,
        }
    }

    /// Set compression flag
    pub fn with_compression(mut self, compressed: bool) -> Self {
        self.compressed = compressed;
        self
    }

    /// Set data type
    pub fn with_type(mut self, data_type: DataType) -> Self {
        self.data_type = data_type;
        self
    }
}

/// RPC call type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcType {
    /// Unary: single request, single response
    Unary,
    /// Server streaming: single request, multiple responses
    ServerStreaming,
    /// Client streaming: multiple requests, single response
    ClientStreaming,
    /// Bidirectional streaming: multiple requests, multiple responses
    BidirectionalStreaming,
}

/// Pending call tracking
struct PendingCall {
    /// Call ID
    call_id: u64,
    /// Stream ID
    stream_id: StreamId,
    /// Response channel
    response_tx: oneshot::Sender<RpcResponse>,
    /// Start time
    start_time: Instant,
    /// Timeout
    timeout: Duration,
    /// RPC type
    rpc_type: RpcType,
}

/// RPC response
#[derive(Debug)]
pub struct RpcResponse {
    /// Response header
    pub header: RpcResponseHeader,
    /// Response data
    pub data: Vec<u8>,
}

/// RPC client for making calls
pub struct RpcClient {
    /// Endpoint URL
    endpoint: String,
    /// Stream manager
    stream_manager: Arc<RwLock<StreamManager>>,
    /// Pending calls
    pending_calls: HashMap<u64, PendingCall>,
    /// Next call ID
    next_call_id: u64,
    /// Default timeout
    default_timeout: Duration,
    /// Flow controller
    flow_controller: FlowController,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            stream_manager: Arc::new(RwLock::new(StreamManager::new(true))),
            pending_calls: HashMap::new(),
            next_call_id: 1,
            default_timeout: Duration::from_secs(30),
            flow_controller: FlowController::default(),
        }
    }

    /// Get endpoint
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Generate next call ID
    fn next_call_id(&mut self) -> u64 {
        let id = self.next_call_id;
        self.next_call_id += 1;
        id
    }

    /// Make a unary RPC call
    pub async fn call_unary(&mut self, header: RpcHeader, request: Vec<u8>) -> Result<RpcResponse> {
        let call_id = header.call_id;
        let timeout = Duration::from_millis(header.timeout_ms as u64);

        // Create stream
        let stream_id = self.stream_manager.write().await.create_stream()?;
        
        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();
        
        // Track pending call
        self.pending_calls.insert(call_id, PendingCall {
            call_id,
            stream_id,
            response_tx,
            start_time: Instant::now(),
            timeout,
            rpc_type: RpcType::Unary,
        });

        // Open stream
        self.stream_manager.write().await.open_stream(stream_id)?;

        // Send headers frame
        let header_bytes = self.serialize_header(&header)?;
        let headers_frame = Frame::headers(stream_id, header_bytes);
        
        // Send data frame with END_STREAM
        let mut flags = FrameFlags::empty();
        flags.set_end_stream();
        let data_frame = Frame::data(stream_id, request, false);

        // Wait for response with timeout
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(response)) => {
                // Close stream
                self.stream_manager.write().await.close_stream(stream_id)?;
                Ok(response)
            }
            Ok(Err(_)) => {
                // Channel closed (error)
                self.stream_manager.write().await.cancel_stream(
                    stream_id,
                    1,
                    "Response channel closed".to_string()
                );
                Err(Error::Rpc("Response channel closed".to_string()))
            }
            Err(_) => {
                // Timeout
                self.stream_manager.write().await.cancel_stream(
                    stream_id,
                    2,
                    "Timeout".to_string()
                );
                Err(Error::Timeout(header.timeout_ms))
            }
        }
    }

    /// Make an RPC call (simple interface)
    pub async fn call(&self, method: &str, request: Vec<u8>) -> Result<Vec<u8>> {
        // Simplified call for backward compatibility
        let header = RpcHeader::new(method.to_string(), self.next_call_id, String::new());
        Ok(request) // Placeholder
    }

    /// Start a server streaming RPC
    pub async fn start_server_stream(
        &mut self,
        header: RpcHeader,
        request: Vec<u8>,
    ) -> Result<RpcStreamReceiver> {
        let stream_id = self.stream_manager.write().await.create_stream()?;
        self.stream_manager.write().await.open_stream(stream_id)?;

        // Create receiver channel
        let (tx, rx) = mpsc::channel(64);
        
        Ok(RpcStreamReceiver {
            stream_id,
            receiver: rx,
            stream_manager: self.stream_manager.clone(),
        })
    }

    /// Start a bidirectional streaming RPC
    pub async fn start_bidirectional_stream(
        &mut self,
        header: RpcHeader,
    ) -> Result<(RpcStreamSender, RpcStreamReceiver)> {
        let stream_id = self.stream_manager.write().await.create_stream()?;
        self.stream_manager.write().await.open_stream(stream_id)?;

        // Create channels
        let (send_tx, send_rx) = mpsc::channel(64);
        let (recv_tx, recv_rx) = mpsc::channel(64);

        let sender = RpcStreamSender {
            stream_id,
            sender: send_tx,
            stream_manager: self.stream_manager.clone(),
        };

        let receiver = RpcStreamReceiver {
            stream_id,
            receiver: recv_rx,
            stream_manager: self.stream_manager.clone(),
        };

        Ok((sender, receiver))
    }

    /// Serialize header to bytes
    fn serialize_header(&self, header: &RpcHeader) -> Result<Vec<u8>> {
        // Use JSON for now, will be replaced with Protobuf
        Ok(serde_json::to_vec(header)?)
    }

    /// Set default timeout
    pub fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }
}

/// RPC stream sender
pub struct RpcStreamSender {
    /// Stream ID
    stream_id: StreamId,
    /// Send channel
    sender: mpsc::Sender<Vec<u8>>,
    /// Stream manager reference
    stream_manager: Arc<RwLock<StreamManager>>,
}

impl RpcStreamSender {
    /// Send data on the stream
    pub async fn send(&self, data: Vec<u8>) -> Result<()> {
        self.sender.send(data).await
            .map_err(|_| Error::Rpc("Stream send failed".to_string()))?;
        Ok(())
    }

    /// Close the send side
    pub async fn close(&self) -> Result<()> {
        // Send END_STREAM signal
        self.stream_manager.write().await.close_stream(self.stream_id)?;
        Ok(())
    }

    /// Get stream ID
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

/// RPC stream receiver
pub struct RpcStreamReceiver {
    /// Stream ID
    stream_id: StreamId,
    /// Receive channel
    receiver: mpsc::Receiver<Vec<u8>>,
    /// Stream manager reference
    stream_manager: Arc<RwLock<StreamManager>>,
}

impl RpcStreamReceiver {
    /// Receive data from the stream
    pub async fn recv(&mut self) -> Result<Option<Vec<u8>>> {
        match self.receiver.recv().await {
            Some(data) => Ok(Some(data)),
            None => Ok(None),
        }
    }

    /// Close the receive side
    pub async fn close(&self) -> Result<()> {
        self.stream_manager.write().await.close_stream_remote(self.stream_id)?;
        Ok(())
    }

    /// Get stream ID
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

/// RPC method handler
pub type MethodHandler = Box<dyn Fn(RpcHeader, Vec<u8>) -> Result<RpcResponse> + Send + Sync>;

/// Streaming method handler
pub type StreamingHandler = Box<dyn Fn(RpcHeader, mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) -> Result<()> + Send + Sync>;

/// RPC server for handling calls
pub struct RpcServer {
    /// Registered unary methods
    methods: HashMap<String, MethodHandler>,
    /// Registered streaming methods
    streaming_methods: HashMap<String, StreamingHandler>,
    /// Stream manager
    stream_manager: StreamManager,
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
            streaming_methods: HashMap::new(),
            stream_manager: StreamManager::new(false),
        }
    }

    /// Register a unary method handler
    pub fn register<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(RpcHeader, Vec<u8>) -> Result<RpcResponse> + Send + Sync + 'static,
    {
        self.methods.insert(method.to_string(), Box::new(handler));
    }

    /// Register a streaming method handler
    pub fn register_streaming<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(RpcHeader, mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) -> Result<()> + Send + Sync + 'static,
    {
        self.streaming_methods.insert(method.to_string(), Box::new(handler));
    }

    /// Handle an incoming frame
    pub async fn handle_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        match frame.header.frame_type {
            FrameType::Headers => {
                self.handle_headers_frame(frame)
            }
            FrameType::Data => {
                self.handle_data_frame(frame)
            }
            FrameType::EndStream => {
                self.handle_end_stream_frame(frame)
            }
            FrameType::Cancel => {
                self.handle_cancel_frame(frame)
            }
            FrameType::Ping => {
                // Respond with ACK
                let payload = frame.payload.clone();
                Ok(Some(Frame::ping(
                    [payload[0], payload[1], payload[2], payload[3],
                     payload[4], payload[5], payload[6], payload[7]],
                    true
                )))
            }
            _ => Ok(None)
        }
    }

    /// Handle headers frame
    fn handle_headers_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        let stream_id = frame.header.stream_id;
        
        // Parse header
        let header: RpcHeader = serde_json::from_slice(&frame.payload)
            .map_err(|e| Error::Protocol(format!("Invalid RPC header: {}", e)))?;

        // Check if method exists
        if !self.methods.contains_key(&header.method) {
            let error_response = RpcResponseHeader::error(
                header.call_id,
                RpcStatus::Error,
                "Method not found".to_string()
            );
            return Ok(Some(Frame::error(stream_id, 1, "Method not found")));
        }

        // Create stream if needed
        if !self.stream_manager.has_stream(stream_id) {
            // Note: Server accepts client-initiated streams
        }

        Ok(None)
    }

    /// Handle data frame
    fn handle_data_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        // Data handling would be implemented with full frame processing
        Ok(None)
    }

    /// Handle end stream frame
    fn handle_end_stream_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        let stream_id = frame.header.stream_id;
        self.stream_manager.close_stream_remote(stream_id)?;
        Ok(None)
    }

    /// Handle cancel frame
    fn handle_cancel_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        let stream_id = frame.header.stream_id;
        self.stream_manager.cancel_stream(stream_id, 3, "Cancelled by client".to_string());
        Ok(None)
    }

    /// Check if method is registered
    pub fn has_method(&self, method: &str) -> bool {
        self.methods.contains_key(method) || self.streaming_methods.contains_key(method)
    }

    /// Get registered methods
    pub fn get_methods(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}

/// RPC stream for streaming calls (legacy interface)
pub struct RpcStream {
    /// Stream ID
    pub stream_id: String,
}

impl RpcStream {
    /// Create a new stream
    pub fn new(id: &str) -> Self {
        Self {
            stream_id: id.to_string(),
        }
    }

    /// Send data on the stream
    pub async fn send(&mut self, _data: Vec<u8>) -> Result<()> {
        Ok(())
    }

    /// Receive data from the stream
    pub async fn recv(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_header() {
        let header = RpcHeader::new("translate".to_string(), 1, "did:nexa:abc123".to_string())
            .with_budget(100)
            .with_timeout(5000);
        
        assert_eq!(header.method, "translate");
        assert_eq!(header.call_id, 1);
        assert_eq!(header.budget, 100);
        assert_eq!(header.timeout_ms, 5000);
    }

    #[test]
    fn test_rpc_response_header() {
        let success = RpcResponseHeader::success(1, 50, 100);
        assert_eq!(success.status, RpcStatus::Success);
        assert_eq!(success.actual_cost, 50);
        
        let error = RpcResponseHeader::error(1, RpcStatus::Timeout, "Timeout".to_string());
        assert_eq!(error.status, RpcStatus::Timeout);
        assert!(error.error_message.is_some());
    }

    #[test]
    fn test_rpc_client() {
        let client = RpcClient::new("http://localhost:7070");
        assert_eq!(client.endpoint(), "http://localhost:7070");
    }

    #[test]
    fn test_rpc_server() {
        let mut server = RpcServer::new();
        server.register("test", |_header, data| {
            Ok(RpcResponse {
                header: RpcResponseHeader::success(1, 0, 0),
                data,
            })
        });
        assert!(server.has_method("test"));
        assert!(!server.has_method("unknown"));
    }

    #[test]
    fn test_data_frame() {
        let frame = DataFrame::new(1, 0, b"test".to_vec())
            .with_compression(true)
            .with_type(DataType::Text);
        
        assert_eq!(frame.stream_id, 1);
        assert!(frame.compressed);
        assert_eq!(frame.data_type, DataType::Text);
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();
        assert!(policy.retryable);
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_delay_ms, 100);
    }
}