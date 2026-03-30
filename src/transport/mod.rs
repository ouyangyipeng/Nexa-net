//! Layer 3: Transport & Negotiation Protocol Layer
//!
//! This module implements binary RPC protocol with streaming support
//! for Nexa-net agents.
//!
//! # Components
//!
//! - **Frame**: 12-byte header frame protocol (DATA, HEADERS, WINDOW_UPDATE, etc.)
//! - **Stream**: Multiplexed stream management with flow control
//! - **RPC**: Streaming RPC engine (Unary, Server Streaming, Bidirectional)
//! - **Serialization**: Protobuf/FlatBuffers/JSON with LZ4 compression
//! - **Connection**: Connection pool and session management
//! - **Negotiator**: Dynamic protocol negotiation (SYN-NEXA/ACK-SCHEMA handshake)
//! - **Error Handler**: Error handling, retry, and timeout management
//!
//! # Frame Protocol
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Frame Header (12 bytes)                  │
//! │  ┌────────┬────────┬────────┬────────┬─────────────┐       │
//! │  │ Length │ Type   │ Stream │ Flags  │ Reserved    │       │
//! │  │ 4 bytes│ 1 byte │ 4 bytes│ 1 byte │ 2 bytes     │       │
//! │  └────────┴────────┴────────┴────────┴─────────────┘       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # RPC Models
//!
//! - **Unary**: Single request, single response
//! - **Server Streaming**: Single request, multiple responses
//! - **Client Streaming**: Multiple requests, single response
//! - **Bidirectional Streaming**: Multiple requests, multiple responses
//!
//! # Example
//!
//! ```rust,no_run
//! use nexa_net::transport::{RpcClient, SerializationEngine, SerializationFormat};
//!
//! // Create serialization engine
//! let engine = SerializationEngine::new(SerializationFormat::Json);
//!
//! // Serialize data
//! let data = serde_json::json!({"key": "value"});
//! let serialized = engine.serialize(&data)?;
//!
//! // Make RPC call
//! let client = RpcClient::new("http://localhost:7070");
//! let response = client.call("translate", serialized).await?;
//! # Ok::<(), nexa_net::Error>(())
//! ```

pub mod frame;
pub mod stream;
pub mod negotiator;
pub mod rpc;
pub mod serialization;
pub mod connection;
pub mod error_handler;

// Re-exports from frame module
pub use frame::{
    Frame, FrameFlags, FrameHeader, FrameReader, FrameType, FrameWriter,
};

// Re-exports from stream module
pub use stream::{
    FlowController, Stream, StreamId, StreamManager, StreamState, StreamStats,
};

// Re-exports from rpc module
pub use rpc::{
    DataFrame, DataType, ErrorDetail, ErrorType, MethodHandler, RetryPolicy,
    RpcClient, RpcHeader, RpcResponse, RpcResponseHeader, RpcServer, RpcStatus,
    RpcStream, RpcStreamReceiver, RpcStreamSender, RpcType, StreamingHandler,
};

// Re-exports from serialization module
pub use serialization::{
    compress, decompress, BinarySerializer, CompressionAlgorithm, CompressionLevel,
    Deserializer, FlatBuffersSerializer, JsonSerializer, ProtobufSerializer,
    SchemaCompressor, SerializationEngine, SerializationFormat, Serializer,
    estimate_compression_ratio, should_compress,
};

// Re-exports from connection module
pub use connection::{Connection, ConnectionPool, Session};

// Re-exports from negotiator module
pub use negotiator::{
    Accept, AckSchema, ClientCapabilities, CompressionType, NegotiatedProtocol,
    NegotiationState, Negotiator, Reject, RejectReason, ServerCapabilities,
    ServerNegotiator, SynNexa,
};

// Re-exports from error_handler module
pub use error_handler::{ErrorHandler, RetryPolicy as ErrorHandlerRetryPolicy, RetryResult};