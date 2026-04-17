//! Frame Protocol Implementation
//!
//! Implements the 12-byte header frame format as specified in TRANSPORT_LAYER.md.
//!
//! # Frame Format
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

use crate::error::{Error, Result};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::{Read, Write};

/// Frame type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum FrameType {
    /// Data frame - carries actual message payload
    Data = 0,
    /// Headers frame - carries RPC headers (method, metadata)
    Headers = 1,
    /// Priority frame - sets stream priority
    Priority = 2,
    /// End stream frame - signals end of stream
    EndStream = 3,
    /// Window update frame - flow control
    WindowUpdate = 4,
    /// Ping frame - keepalive/latency measurement
    Ping = 5,
    /// Cancel frame - cancel a stream
    Cancel = 6,
    /// Error frame - error notification
    Error = 7,
}

#[allow(clippy::derivable_impls)]
impl Default for FrameType {
    fn default() -> Self {
        FrameType::Data
    }
}

/// Frame flags enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameFlags(u8);

impl FrameFlags {
    /// No flags
    pub const NONE: u8 = 0;
    /// End of stream flag
    pub const END_STREAM: u8 = 0x01;
    /// End of frame flag
    pub const END_FRAME: u8 = 0x02;
    /// Compressed flag
    pub const COMPRESSED: u8 = 0x04;
    /// End headers flag
    pub const END_HEADERS: u8 = 0x08;
    /// ACK flag (for Ping responses)
    pub const ACK: u8 = 0x10;

    /// Create new flags
    pub fn new(flags: u8) -> Self {
        Self(flags)
    }

    /// Create empty flags
    pub fn empty() -> Self {
        Self(0)
    }

    /// Get raw flags value
    pub fn raw(&self) -> u8 {
        self.0
    }

    /// Check if END_STREAM flag is set
    pub fn is_end_stream(&self) -> bool {
        (self.0 & Self::END_STREAM) != 0
    }

    /// Check if COMPRESSED flag is set
    pub fn is_compressed(&self) -> bool {
        (self.0 & Self::COMPRESSED) != 0
    }

    /// Check if END_HEADERS flag is set
    pub fn is_end_headers(&self) -> bool {
        (self.0 & Self::END_HEADERS) != 0
    }

    /// Check if ACK flag is set
    pub fn is_ack(&self) -> bool {
        (self.0 & Self::ACK) != 0
    }

    /// Set END_STREAM flag
    pub fn set_end_stream(&mut self) {
        self.0 |= Self::END_STREAM;
    }

    /// Set COMPRESSED flag
    pub fn set_compressed(&mut self) {
        self.0 |= Self::COMPRESSED;
    }

    /// Set END_HEADERS flag
    pub fn set_end_headers(&mut self) {
        self.0 |= Self::END_HEADERS;
    }

    /// Set ACK flag
    pub fn set_ack(&mut self) {
        self.0 |= Self::ACK;
    }
}

impl Default for FrameFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Frame header structure (12 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    /// Payload length (4 bytes, big-endian)
    pub length: u32,
    /// Frame type (1 byte)
    pub frame_type: FrameType,
    /// Stream ID (4 bytes, big-endian)
    pub stream_id: u32,
    /// Flags (1 byte)
    pub flags: FrameFlags,
    /// Reserved (2 bytes)
    pub reserved: u16,
}

impl FrameHeader {
    /// Header size in bytes
    pub const SIZE: usize = 12;

    /// Create a new frame header
    pub fn new(length: u32, frame_type: FrameType, stream_id: u32, flags: FrameFlags) -> Self {
        Self {
            length,
            frame_type,
            stream_id,
            flags,
            reserved: 0,
        }
    }

    /// Create a data frame header
    pub fn data(stream_id: u32, length: u32, flags: FrameFlags) -> Self {
        Self::new(length, FrameType::Data, stream_id, flags)
    }

    /// Create a headers frame header
    pub fn headers(stream_id: u32, length: u32) -> Self {
        Self::new(length, FrameType::Headers, stream_id, FrameFlags::empty())
    }

    /// Create a window update frame header
    pub fn window_update(stream_id: u32, _increment: u32) -> Self {
        Self::new(4, FrameType::WindowUpdate, stream_id, FrameFlags::empty())
    }

    /// Create a ping frame header
    pub fn ping(is_ack: bool) -> Self {
        let flags = if is_ack {
            FrameFlags::new(FrameFlags::ACK)
        } else {
            FrameFlags::empty()
        };
        Self::new(8, FrameType::Ping, 0, flags)
    }

    /// Create a cancel frame header
    pub fn cancel(stream_id: u32) -> Self {
        Self::new(0, FrameType::Cancel, stream_id, FrameFlags::empty())
    }

    /// Create an error frame header
    pub fn error(stream_id: u32, length: u32) -> Self {
        Self::new(length, FrameType::Error, stream_id, FrameFlags::empty())
    }

    /// Encode header to bytes (big-endian)
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.length.to_be_bytes());
        buf.push(self.frame_type as u8);
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.push(self.flags.raw());
        buf.extend_from_slice(&self.reserved.to_be_bytes());
        buf
    }

    /// Decode header from bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::Protocol("Frame header too short".to_string()));
        }

        let length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let frame_type = FrameType::from_u8(data[4])
            .ok_or_else(|| Error::Protocol(format!("Invalid frame type: {}", data[4])))?;
        let stream_id = u32::from_be_bytes([data[5], data[6], data[7], data[8]]);
        let flags = FrameFlags::new(data[9]);
        let reserved = u16::from_be_bytes([data[10], data[11]]);

        Ok(Self {
            length,
            frame_type,
            stream_id,
            flags,
            reserved,
        })
    }
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self::new(0, FrameType::Data, 0, FrameFlags::empty())
    }
}

/// Complete frame with header and payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Frame header
    pub header: FrameHeader,
    /// Frame payload
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new frame
    pub fn new(header: FrameHeader, payload: Vec<u8>) -> Self {
        Self { header, payload }
    }

    /// Create a data frame
    pub fn data(stream_id: u32, payload: Vec<u8>, compressed: bool) -> Self {
        let mut flags = FrameFlags::empty();
        if compressed {
            flags.set_compressed();
        }
        let header = FrameHeader::data(stream_id, payload.len() as u32, flags);
        Self::new(header, payload)
    }

    /// Create a headers frame
    pub fn headers(stream_id: u32, payload: Vec<u8>) -> Self {
        let header = FrameHeader::headers(stream_id, payload.len() as u32);
        Self::new(header, payload)
    }

    /// Create an end stream frame
    pub fn end_stream(stream_id: u32) -> Self {
        let flags = FrameFlags::new(FrameFlags::END_STREAM);
        let header = FrameHeader::data(stream_id, 0, flags);
        Self::new(header, Vec::new())
    }

    /// Create a window update frame
    pub fn window_update(stream_id: u32, increment: u32) -> Self {
        let header = FrameHeader::window_update(stream_id, increment);
        let payload = increment.to_be_bytes().to_vec();
        Self::new(header, payload)
    }

    /// Create a ping frame
    pub fn ping(payload: [u8; 8], is_ack: bool) -> Self {
        let header = FrameHeader::ping(is_ack);
        Self::new(header, payload.to_vec())
    }

    /// Create a cancel frame
    pub fn cancel(stream_id: u32) -> Self {
        let header = FrameHeader::cancel(stream_id);
        Self::new(header, Vec::new())
    }

    /// Create an error frame
    pub fn error(stream_id: u32, error_code: u32, message: &str) -> Self {
        let mut payload = Vec::with_capacity(4 + message.len());
        payload.extend_from_slice(&error_code.to_be_bytes());
        payload.extend_from_slice(message.as_bytes());
        let header = FrameHeader::error(stream_id, payload.len() as u32);
        Self::new(header, payload)
    }

    /// Encode frame to bytes
    ///
    /// Pre-allocates the full frame size (header + payload) to avoid
    /// reallocation when extending the header buffer with payload data.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(FrameHeader::SIZE + self.payload.len());
        buf.extend_from_slice(&self.header.encode());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode frame from bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        let header = FrameHeader::decode(data)?;
        let payload_start = FrameHeader::SIZE;
        let payload_end = payload_start + header.length as usize;

        if data.len() < payload_end {
            return Err(Error::Protocol(format!(
                "Frame payload truncated: expected {} bytes, got {}",
                payload_end,
                data.len()
            )));
        }

        let payload = data[payload_start..payload_end].to_vec();
        Ok(Self { header, payload })
    }

    /// Check if this is a data frame
    pub fn is_data(&self) -> bool {
        self.header.frame_type == FrameType::Data
    }

    /// Check if this is a headers frame
    pub fn is_headers(&self) -> bool {
        self.header.frame_type == FrameType::Headers
    }

    /// Check if this frame signals end of stream
    pub fn is_end_of_stream(&self) -> bool {
        self.header.flags.is_end_stream() || self.header.frame_type == FrameType::EndStream
    }

    /// Check if payload is compressed
    pub fn is_compressed(&self) -> bool {
        self.header.flags.is_compressed()
    }

    /// Get total frame size
    pub fn total_size(&self) -> usize {
        FrameHeader::SIZE + self.payload.len()
    }
}

/// Frame reader for streaming frame parsing
pub struct FrameReader<R: Read> {
    reader: R,
    #[allow(dead_code)]
    buffer: Vec<u8>,
}

impl<R: Read> FrameReader<R> {
    /// Create a new frame reader
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::with_capacity(64 * 1024),
        }
    }

    /// Read next frame
    pub fn read_frame(&mut self) -> Result<Option<Frame>> {
        // Read header
        let mut header_buf = [0u8; FrameHeader::SIZE];
        let bytes_read = self.reader.read(&mut header_buf)?;

        if bytes_read == 0 {
            return Ok(None); // EOF
        }

        if bytes_read < FrameHeader::SIZE {
            return Err(Error::Protocol("Incomplete frame header".to_string()));
        }

        let header = FrameHeader::decode(&header_buf)?;

        // Read payload
        let mut payload = vec![0u8; header.length as usize];
        if header.length > 0 {
            self.reader.read_exact(&mut payload)?;
        }

        Ok(Some(Frame::new(header, payload)))
    }
}

/// Frame writer for streaming frame encoding
pub struct FrameWriter<W: Write> {
    writer: W,
}

impl<W: Write> FrameWriter<W> {
    /// Create a new frame writer
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a frame
    pub fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        let encoded = frame.encode();
        self.writer.write_all(&encoded)?;
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_flags() {
        let mut flags = FrameFlags::empty();
        assert!(!flags.is_end_stream());
        assert!(!flags.is_compressed());

        flags.set_end_stream();
        flags.set_compressed();
        assert!(flags.is_end_stream());
        assert!(flags.is_compressed());
        assert_eq!(flags.raw(), FrameFlags::END_STREAM | FrameFlags::COMPRESSED);
    }

    #[test]
    fn test_frame_header_encode_decode() {
        let header = FrameHeader::data(1, 100, FrameFlags::new(FrameFlags::COMPRESSED));
        let encoded = header.encode();
        assert_eq!(encoded.len(), FrameHeader::SIZE);

        let decoded = FrameHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.length, 100);
        assert_eq!(decoded.frame_type, FrameType::Data);
        assert_eq!(decoded.stream_id, 1);
        assert!(decoded.flags.is_compressed());
    }

    #[test]
    fn test_frame_encode_decode() {
        let payload = b"test payload".to_vec();
        let frame = Frame::data(1, payload.clone(), true);

        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap();

        assert_eq!(decoded.header.frame_type, FrameType::Data);
        assert_eq!(decoded.header.stream_id, 1);
        assert!(decoded.is_compressed());
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_ping_frame() {
        let ping_data = [0u8; 8];
        let ping = Frame::ping(ping_data, false);
        assert_eq!(ping.header.frame_type, FrameType::Ping);
        assert!(!ping.header.flags.is_ack());

        let ack = Frame::ping(ping_data, true);
        assert!(ack.header.flags.is_ack());
    }

    #[test]
    fn test_window_update_frame() {
        let frame = Frame::window_update(1, 1024);
        assert_eq!(frame.header.frame_type, FrameType::WindowUpdate);
        assert_eq!(frame.header.stream_id, 1);

        let increment = u32::from_be_bytes([
            frame.payload[0],
            frame.payload[1],
            frame.payload[2],
            frame.payload[3],
        ]);
        assert_eq!(increment, 1024);
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_frame_header_decode_too_short() {
        let short_data = [0u8; 5]; // Less than 12 bytes
        let result = FrameHeader::decode(&short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_header_decode_invalid_frame_type() {
        let mut data = Vec::new();
        data.extend_from_slice(&100u32.to_be_bytes()); // length
        data.push(255u8); // Invalid frame type
        data.extend_from_slice(&1u32.to_be_bytes()); // stream_id
        data.push(0u8); // flags
        data.extend_from_slice(&0u16.to_be_bytes()); // reserved

        let result = FrameHeader::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_decode_truncated_payload() {
        // Header says 100 bytes payload but we only provide 12 bytes (header only)
        let mut data = Vec::new();
        data.extend_from_slice(&100u32.to_be_bytes()); // length = 100
        data.push(0u8); // Data frame type
        data.extend_from_slice(&1u32.to_be_bytes()); // stream_id
        data.push(0u8); // flags
        data.extend_from_slice(&0u16.to_be_bytes()); // reserved

        let result = Frame::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_payload_frame() {
        let frame = Frame::data(1, Vec::new(), false);
        assert_eq!(frame.payload.len(), 0);
        assert_eq!(frame.total_size(), FrameHeader::SIZE);

        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(decoded.payload.len(), 0);
    }

    #[test]
    fn test_end_stream_frame() {
        let frame = Frame::end_stream(42);
        assert!(frame.is_end_of_stream());
        assert_eq!(frame.payload.len(), 0);
    }

    #[test]
    fn test_cancel_frame() {
        let frame = Frame::cancel(42);
        assert_eq!(frame.header.frame_type, FrameType::Cancel);
        assert_eq!(frame.header.stream_id, 42);
        assert_eq!(frame.payload.len(), 0);
    }

    #[test]
    fn test_error_frame() {
        let frame = Frame::error(42, 500, "internal error");
        assert_eq!(frame.header.frame_type, FrameType::Error);
        assert_eq!(frame.header.stream_id, 42);
        assert!(!frame.payload.is_empty());
    }

    #[test]
    fn test_frame_flags_all_set() {
        let flags = FrameFlags::new(
            FrameFlags::END_STREAM
                | FrameFlags::END_FRAME
                | FrameFlags::COMPRESSED
                | FrameFlags::END_HEADERS
                | FrameFlags::ACK,
        );
        assert!(flags.is_end_stream());
        assert!(flags.is_compressed());
        assert!(flags.is_end_headers());
        assert!(flags.is_ack());
    }

    #[test]
    fn test_frame_header_default() {
        let header = FrameHeader::default();
        assert_eq!(header.length, 0);
        assert_eq!(header.frame_type, FrameType::Data);
        assert_eq!(header.stream_id, 0);
        assert_eq!(header.reserved, 0);
    }

    #[test]
    fn test_headers_frame() {
        let payload = b"headers data".to_vec();
        let frame = Frame::headers(1, payload.clone());
        assert!(frame.is_headers());
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn test_large_payload_frame() {
        let payload: Vec<u8> = (0..65535).map(|i| (i % 256) as u8).collect();
        let frame = Frame::data(1, payload.clone(), false);

        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_frame_type_values() {
        assert_eq!(FrameType::Data as u8, 0);
        assert_eq!(FrameType::Headers as u8, 1);
        assert_eq!(FrameType::Priority as u8, 2);
        assert_eq!(FrameType::EndStream as u8, 3);
        assert_eq!(FrameType::WindowUpdate as u8, 4);
        assert_eq!(FrameType::Ping as u8, 5);
        assert_eq!(FrameType::Cancel as u8, 6);
        assert_eq!(FrameType::Error as u8, 7);
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Frame encode→decode round-trip with arbitrary payload bytes
        #[test]
        fn proptest_frame_roundtrip(
            payload in prop::collection::vec(any::<u8>(), 0..1000),
            stream_id in 0u32..1000u32,
            compressed in any::<bool>(),
        ) {
            let frame = Frame::data(stream_id as u32, payload.clone(), compressed);
            let encoded = frame.encode();
            let decoded = Frame::decode(&encoded).unwrap();

            assert_eq!(decoded.payload, payload);
            assert_eq!(decoded.header.stream_id, stream_id as u32);
            if compressed {
                assert!(decoded.is_compressed());
            } else {
                assert!(!decoded.is_compressed());
            }
        }

        /// Frame header encode→decode preserves all fields
        #[test]
        fn proptest_frame_header_roundtrip(
            length in any::<u32>(),
            stream_id in any::<u32>(),
            flags_byte in any::<u8>(),
        ) {
            let header = FrameHeader::new(length, FrameType::Data, stream_id, FrameFlags::new(flags_byte));
            let encoded = header.encode();
            let decoded = FrameHeader::decode(&encoded).unwrap();

            assert_eq!(decoded.length, length);
            assert_eq!(decoded.frame_type, FrameType::Data);
            assert_eq!(decoded.stream_id, stream_id);
            assert_eq!(decoded.flags.raw(), flags_byte);
        }
    }
}
