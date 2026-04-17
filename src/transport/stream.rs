//! Stream Management
//!
//! Implements multiplexed stream management with flow control.
//!
//! # Stream States
//!
//! ```text
//! idle -> open -> half_closed_local -> closed
//! idle -> open -> half_closed_remote -> closed
//! ```

use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Stream ID type
pub type StreamId = u32;

/// Stream state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamState {
    #[default]
    /// Stream not yet used
    Idle,
    /// Stream is open for both directions
    Open,
    /// Local side has closed (sent END_STREAM)
    HalfClosedLocal,
    /// Remote side has closed (received END_STREAM)
    HalfClosedRemote,
    /// Stream is fully closed
    Closed,
}

/// Stream statistics
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// Bytes sent on this stream
    pub bytes_sent: u64,
    /// Bytes received on this stream
    pub bytes_received: u64,
    /// Frames sent
    pub frames_sent: u64,
    /// Frames received
    pub frames_received: u64,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
}

impl Default for StreamStats {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            frames_sent: 0,
            frames_received: 0,
            created_at: now,
            last_activity: now,
        }
    }
}

impl StreamStats {
    /// Update sent statistics
    pub fn record_send(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.frames_sent += 1;
        self.last_activity = Instant::now();
    }

    /// Update received statistics
    pub fn record_recv(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.frames_received += 1;
        self.last_activity = Instant::now();
    }

    /// Get stream duration
    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get idle time since last activity
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Individual stream
#[derive(Debug)]
pub struct Stream {
    /// Stream ID
    pub id: StreamId,
    /// Current state
    pub state: StreamState,
    /// Stream statistics
    pub stats: StreamStats,
    /// Send window size (flow control)
    pub send_window: u32,
    /// Receive window size (flow control)
    pub recv_window: u32,
    /// Priority (0-255, higher = more priority)
    pub priority: u8,
    /// Error code if closed due to error
    pub error_code: Option<u32>,
    /// Error message if closed due to error
    pub error_message: Option<String>,
}

impl Stream {
    /// Default initial window size
    pub const DEFAULT_WINDOW: u32 = 65535;
    /// Maximum window size
    pub const MAX_WINDOW: u32 = 1 << 30; // 1GB

    /// Create a new stream
    pub fn new(id: StreamId) -> Self {
        Self {
            id,
            state: StreamState::Idle,
            stats: StreamStats::default(),
            send_window: Self::DEFAULT_WINDOW,
            recv_window: Self::DEFAULT_WINDOW,
            priority: 0,
            error_code: None,
            error_message: None,
        }
    }

    /// Open the stream
    pub fn open(&mut self) -> Result<()> {
        if self.state != StreamState::Idle {
            return Err(Error::Protocol(format!(
                "Cannot open stream {} in state {:?}",
                self.id, self.state
            )));
        }
        self.state = StreamState::Open;
        Ok(())
    }

    /// Close local side (send END_STREAM)
    pub fn close_local(&mut self) -> Result<()> {
        match self.state {
            StreamState::Open => {
                self.state = StreamState::HalfClosedLocal;
            }
            StreamState::HalfClosedRemote => {
                self.state = StreamState::Closed;
            }
            _ => {
                return Err(Error::Protocol(format!(
                    "Cannot close local side of stream {} in state {:?}",
                    self.id, self.state
                )));
            }
        }
        Ok(())
    }

    /// Close remote side (received END_STREAM)
    pub fn close_remote(&mut self) -> Result<()> {
        match self.state {
            StreamState::Open => {
                self.state = StreamState::HalfClosedRemote;
            }
            StreamState::HalfClosedLocal => {
                self.state = StreamState::Closed;
            }
            _ => {
                return Err(Error::Protocol(format!(
                    "Cannot close remote side of stream {} in state {:?}",
                    self.id, self.state
                )));
            }
        }
        Ok(())
    }

    /// Cancel the stream with error
    pub fn cancel(&mut self, error_code: u32, message: String) {
        self.state = StreamState::Closed;
        self.error_code = Some(error_code);
        self.error_message = Some(message);
    }

    /// Check if stream can send data
    pub fn can_send(&self) -> bool {
        matches!(self.state, StreamState::Idle | StreamState::Open) && self.send_window > 0
    }

    /// Check if stream can receive data
    pub fn can_receive(&self) -> bool {
        matches!(
            self.state,
            StreamState::Idle | StreamState::Open | StreamState::HalfClosedLocal
        )
    }

    /// Check if stream is active
    pub fn is_active(&self) -> bool {
        !matches!(self.state, StreamState::Closed)
    }

    /// Update send window
    pub fn update_send_window(&mut self, increment: u32) -> Result<()> {
        let new_window = self.send_window.saturating_add(increment);
        if new_window > Self::MAX_WINDOW {
            return Err(Error::Protocol("Window size overflow".to_string()));
        }
        self.send_window = new_window;
        Ok(())
    }

    /// Consume send window
    pub fn consume_send_window(&mut self, size: u32) -> Result<()> {
        if size > self.send_window {
            return Err(Error::Protocol("Window size exceeded".to_string()));
        }
        self.send_window -= size;
        Ok(())
    }

    /// Consume receive window
    pub fn consume_recv_window(&mut self, size: u32) -> Result<()> {
        if size > self.recv_window {
            return Err(Error::Protocol("Receive window exceeded".to_string()));
        }
        self.recv_window -= size;
        Ok(())
    }

    /// Set stream priority
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority;
    }

    /// Record bytes sent
    pub fn record_send(&mut self, bytes: u64) {
        self.stats.record_send(bytes);
    }

    /// Record bytes received
    pub fn record_recv(&mut self, bytes: u64) {
        self.stats.record_recv(bytes);
    }
}

/// Stream manager for multiplexing
pub struct StreamManager {
    /// Active streams
    streams: HashMap<StreamId, Stream>,
    /// Next stream ID (client-initiated streams use odd numbers)
    next_stream_id: AtomicU32,
    /// Maximum concurrent streams
    max_concurrent_streams: usize,
    /// Initial window size for new streams
    initial_window_size: u32,
    /// Stream timeout
    stream_timeout: Duration,
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new(is_client: bool) -> Self {
        // Client-initiated streams use odd IDs, server-initiated use even
        let initial_id = if is_client { 1 } else { 2 };

        Self {
            streams: HashMap::new(),
            next_stream_id: AtomicU32::new(initial_id),
            max_concurrent_streams: 100,
            initial_window_size: Stream::DEFAULT_WINDOW,
            stream_timeout: Duration::from_secs(300),
        }
    }

    /// Create a new stream
    pub fn create_stream(&mut self) -> Result<StreamId> {
        if self.streams.len() >= self.max_concurrent_streams {
            return Err(Error::Protocol(
                "Maximum concurrent streams exceeded".to_string(),
            ));
        }

        let stream_id = self.next_stream_id.fetch_add(2, Ordering::SeqCst);
        let stream = Stream::new(stream_id);
        self.streams.insert(stream_id, stream);

        Ok(stream_id)
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: StreamId) -> Option<&Stream> {
        self.streams.get(&stream_id)
    }

    /// Get a mutable stream by ID
    pub fn get_stream_mut(&mut self, stream_id: StreamId) -> Option<&mut Stream> {
        self.streams.get_mut(&stream_id)
    }

    /// Open a stream
    pub fn open_stream(&mut self, stream_id: StreamId) -> Result<()> {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .ok_or_else(|| Error::Protocol(format!("Stream {} not found", stream_id)))?;
        stream.open()
    }

    /// Close a stream (local side)
    pub fn close_stream(&mut self, stream_id: StreamId) -> Result<()> {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .ok_or_else(|| Error::Protocol(format!("Stream {} not found", stream_id)))?;
        stream.close_local()
    }

    /// Close a stream (remote side)
    pub fn close_stream_remote(&mut self, stream_id: StreamId) -> Result<()> {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .ok_or_else(|| Error::Protocol(format!("Stream {} not found", stream_id)))?;
        stream.close_remote()
    }

    /// Cancel a stream
    pub fn cancel_stream(&mut self, stream_id: StreamId, error_code: u32, reason: String) {
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.cancel(error_code, reason);
        }
    }

    /// Remove a closed stream
    pub fn remove_stream(&mut self, stream_id: StreamId) -> Option<Stream> {
        if let Some(stream) = self.streams.get(&stream_id) {
            if stream.state == StreamState::Closed {
                return self.streams.remove(&stream_id);
            }
        }
        None
    }

    /// Get all active stream IDs
    pub fn get_active_streams(&self) -> Vec<StreamId> {
        self.streams
            .iter()
            .filter(|(_, s)| s.is_active())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get number of active streams
    pub fn active_stream_count(&self) -> usize {
        self.streams.iter().filter(|(_, s)| s.is_active()).count()
    }

    /// Check if a stream exists
    pub fn has_stream(&self, stream_id: StreamId) -> bool {
        self.streams.contains_key(&stream_id)
    }

    /// Update send window for a stream
    pub fn update_send_window(&mut self, stream_id: StreamId, increment: u32) -> Result<()> {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .ok_or_else(|| Error::Protocol(format!("Stream {} not found", stream_id)))?;
        stream.update_send_window(increment)
    }

    /// Clean up expired streams
    pub fn cleanup_expired(&mut self) -> Vec<StreamId> {
        let expired: Vec<StreamId> = self
            .streams
            .iter()
            .filter(|(_, s)| {
                s.state == StreamState::Closed || s.stats.idle_time() > self.stream_timeout
            })
            .map(|(id, _)| *id)
            .collect();

        for id in &expired {
            self.streams.remove(id);
        }

        expired
    }

    /// Set maximum concurrent streams
    pub fn set_max_concurrent(&mut self, max: usize) {
        self.max_concurrent_streams = max;
    }

    /// Set initial window size
    pub fn set_initial_window(&mut self, window: u32) {
        self.initial_window_size = window;
    }

    /// Set stream timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.stream_timeout = timeout;
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new(true) // Default to client mode
    }
}

/// Flow controller for managing window sizes
pub struct FlowController {
    /// Current window size
    window_size: u32,
    /// Bytes sent but not yet acknowledged
    bytes_in_flight: u32,
    /// Bytes received but not yet window-updated
    bytes_received: u32,
    /// Threshold for sending WINDOW_UPDATE (half of window)
    update_threshold: u32,
}

impl FlowController {
    /// Create a new flow controller
    pub fn new(initial_window: u32) -> Self {
        Self {
            window_size: initial_window,
            bytes_in_flight: 0,
            bytes_received: 0,
            update_threshold: initial_window / 2,
        }
    }

    /// Check if we can send a given number of bytes
    pub fn can_send(&self, size: u32) -> bool {
        self.window_size - self.bytes_in_flight >= size
    }

    /// Get available window for sending
    pub fn available_window(&self) -> u32 {
        self.window_size - self.bytes_in_flight
    }

    /// Record bytes sent
    pub fn on_send(&mut self, size: u32) -> Result<()> {
        if !self.can_send(size) {
            return Err(Error::Protocol("Flow control window exceeded".to_string()));
        }
        self.bytes_in_flight += size;
        Ok(())
    }

    /// Record window update received (acknowledgment)
    pub fn on_window_update(&mut self, increment: u32) {
        self.window_size = self.window_size.saturating_add(increment);
        // Reduce bytes in flight as they're acknowledged
        self.bytes_in_flight = self.bytes_in_flight.saturating_sub(increment);
    }

    /// Record bytes received
    pub fn on_receive(&mut self, size: u32) {
        self.bytes_received += size;
    }

    /// Check if we need to send WINDOW_UPDATE
    pub fn needs_window_update(&self) -> bool {
        self.bytes_received >= self.update_threshold
    }

    /// Get window update increment to send
    pub fn get_window_update_increment(&mut self) -> u32 {
        let increment = self.bytes_received;
        self.bytes_received = 0;
        increment
    }

    /// Reset the controller
    pub fn reset(&mut self, initial_window: u32) {
        self.window_size = initial_window;
        self.bytes_in_flight = 0;
        self.bytes_received = 0;
        self.update_threshold = initial_window / 2;
    }
}

impl Default for FlowController {
    fn default() -> Self {
        Self::new(Stream::DEFAULT_WINDOW)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_state_transitions() {
        let mut stream = Stream::new(1);
        assert_eq!(stream.state, StreamState::Idle);

        stream.open().unwrap();
        assert_eq!(stream.state, StreamState::Open);

        stream.close_local().unwrap();
        assert_eq!(stream.state, StreamState::HalfClosedLocal);

        stream.close_remote().unwrap();
        assert_eq!(stream.state, StreamState::Closed);
    }

    #[test]
    fn test_stream_window_management() {
        let mut stream = Stream::new(1);

        assert!(stream.can_send());
        assert_eq!(stream.send_window, Stream::DEFAULT_WINDOW);

        stream.consume_send_window(1000).unwrap();
        assert_eq!(stream.send_window, Stream::DEFAULT_WINDOW - 1000);

        stream.update_send_window(500).unwrap();
        assert_eq!(stream.send_window, Stream::DEFAULT_WINDOW - 1000 + 500);
    }

    #[test]
    fn test_stream_manager_create() {
        let mut manager = StreamManager::new(true);

        let id1 = manager.create_stream().unwrap();
        assert_eq!(id1, 1); // First client stream

        let id2 = manager.create_stream().unwrap();
        assert_eq!(id2, 3); // Second client stream (odd numbers)

        assert!(manager.has_stream(id1));
        assert!(manager.has_stream(id2));
    }

    #[test]
    fn test_flow_controller() {
        let mut controller = FlowController::new(65535);

        assert!(controller.can_send(1000));
        controller.on_send(1000).unwrap();
        assert_eq!(controller.available_window(), 65535 - 1000);

        controller.on_window_update(500);
        assert!(controller.available_window() >= 65535 - 1000 + 500);

        controller.on_receive(40000);
        assert!(controller.needs_window_update());
    }

    #[test]
    fn test_stream_stats() {
        let mut stats = StreamStats::default();

        stats.record_send(100);
        stats.record_send(200);
        stats.record_recv(150);

        assert_eq!(stats.bytes_sent, 300);
        assert_eq!(stats.bytes_received, 150);
        assert_eq!(stats.frames_sent, 2);
        assert_eq!(stats.frames_received, 1);
    }
}
