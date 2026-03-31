//! Audit Logging
//!
//! Security audit logging for tracking key operations and authentication events.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication method used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Ed25519 signature
    Signature,
    /// mTLS certificate
    MTls,
    /// DID authentication
    DidAuth,
    /// Token-based
    Token,
}

/// Channel state for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelState {
    Open,
    Active,
    Closing,
    Closed,
    Disputed,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    /// Key was generated
    KeyGenerated {
        key_id: String,
        key_type: String,
        timestamp: DateTime<Utc>,
    },

    /// Key was rotated
    KeyRotated {
        key_id: String,
        old_version: u32,
        new_version: u32,
        timestamp: DateTime<Utc>,
    },

    /// Key was accessed
    KeyAccessed {
        key_id: String,
        accessor: String,
        operation: String,
        timestamp: DateTime<Utc>,
    },

    /// Successful authentication
    AuthenticationSuccess {
        did: String,
        method: AuthMethod,
        ip_address: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Failed authentication attempt
    AuthenticationFailure {
        did: String,
        reason: String,
        ip_address: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Channel opened
    ChannelOpened {
        channel_id: String,
        party_a: String,
        party_b: String,
        timestamp: DateTime<Utc>,
    },

    /// Channel closed
    ChannelClosed {
        channel_id: String,
        final_state: ChannelState,
        final_balance_a: u64,
        final_balance_b: u64,
        timestamp: DateTime<Utc>,
    },

    /// Rate limit exceeded
    RateLimitExceeded {
        identifier: String,
        limit_type: String,
        current_rate: u32,
        limit: u32,
        timestamp: DateTime<Utc>,
    },

    /// Security violation detected
    SecurityViolation {
        violation_type: String,
        details: String,
        severity: String,
        timestamp: DateTime<Utc>,
    },
}

impl AuditEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            AuditEvent::KeyGenerated { timestamp, .. } => *timestamp,
            AuditEvent::KeyRotated { timestamp, .. } => *timestamp,
            AuditEvent::KeyAccessed { timestamp, .. } => *timestamp,
            AuditEvent::AuthenticationSuccess { timestamp, .. } => *timestamp,
            AuditEvent::AuthenticationFailure { timestamp, .. } => *timestamp,
            AuditEvent::ChannelOpened { timestamp, .. } => *timestamp,
            AuditEvent::ChannelClosed { timestamp, .. } => *timestamp,
            AuditEvent::RateLimitExceeded { timestamp, .. } => *timestamp,
            AuditEvent::SecurityViolation { timestamp, .. } => *timestamp,
        }
    }

    /// Get event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            AuditEvent::KeyGenerated { .. } => "key_generated",
            AuditEvent::KeyRotated { .. } => "key_rotated",
            AuditEvent::KeyAccessed { .. } => "key_accessed",
            AuditEvent::AuthenticationSuccess { .. } => "auth_success",
            AuditEvent::AuthenticationFailure { .. } => "auth_failure",
            AuditEvent::ChannelOpened { .. } => "channel_opened",
            AuditEvent::ChannelClosed { .. } => "channel_closed",
            AuditEvent::RateLimitExceeded { .. } => "rate_limit_exceeded",
            AuditEvent::SecurityViolation { .. } => "security_violation",
        }
    }
}

/// Audit sink trait for custom log destinations
pub trait AuditSink: Send + Sync {
    /// Log an audit event
    fn log(&self, event: AuditEvent) -> Result<()>;
}

/// In-memory audit sink for testing
pub struct MemoryAuditSink {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    max_events: usize,
}

impl MemoryAuditSink {
    /// Create a new memory sink
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events,
        }
    }

    /// Get all events
    pub async fn get_events(&self) -> Vec<AuditEvent> {
        self.events.read().await.clone()
    }

    /// Get events by type
    pub async fn get_events_by_type(&self, event_type: &str) -> Vec<AuditEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.event_type() == event_type)
            .cloned()
            .collect()
    }

    /// Clear all events
    pub async fn clear(&self) {
        self.events.write().await.clear();
    }
}

impl AuditSink for MemoryAuditSink {
    fn log(&self, event: AuditEvent) -> Result<()> {
        let events = self.events.clone();
        let max = self.max_events;

        // Use try_write to avoid blocking in async context
        if let Ok(mut events) = events.try_write() {
            if events.len() >= max {
                events.remove(0);
            }
            events.push(event);
        }

        Ok(())
    }
}

/// Logging audit sink (outputs to tracing)
pub struct LoggingAuditSink;

impl AuditSink for LoggingAuditSink {
    fn log(&self, event: AuditEvent) -> Result<()> {
        tracing::info!(
            event_type = event.event_type(),
            timestamp = %event.timestamp(),
            "Audit event: {:?}",
            event
        );
        Ok(())
    }
}

/// Audit logger
pub struct AuditLogger {
    sink: Arc<dyn AuditSink>,
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger with a sink
    pub fn new(sink: Arc<dyn AuditSink>) -> Self {
        Self {
            sink,
            enabled: true,
        }
    }

    /// Create with logging sink
    pub fn with_logging() -> Self {
        Self::new(Arc::new(LoggingAuditSink))
    }

    /// Create with memory sink (for testing)
    pub fn with_memory(max_events: usize) -> Self {
        Self::new(Arc::new(MemoryAuditSink::new(max_events)))
    }

    /// Enable or disable logging
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Log an event
    pub fn log(&self, event: AuditEvent) -> Result<()> {
        if self.enabled {
            self.sink.log(event)?;
        }
        Ok(())
    }

    /// Log key generation
    pub fn log_key_generated(&self, key_id: &str, key_type: &str) -> Result<()> {
        self.log(AuditEvent::KeyGenerated {
            key_id: key_id.to_string(),
            key_type: key_type.to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Log key rotation
    pub fn log_key_rotated(&self, key_id: &str, old_version: u32, new_version: u32) -> Result<()> {
        self.log(AuditEvent::KeyRotated {
            key_id: key_id.to_string(),
            old_version,
            new_version,
            timestamp: Utc::now(),
        })
    }

    /// Log authentication success
    pub fn log_auth_success(&self, did: &str, method: AuthMethod, ip: Option<&str>) -> Result<()> {
        self.log(AuditEvent::AuthenticationSuccess {
            did: did.to_string(),
            method,
            ip_address: ip.map(|s| s.to_string()),
            timestamp: Utc::now(),
        })
    }

    /// Log authentication failure
    pub fn log_auth_failure(&self, did: &str, reason: &str, ip: Option<&str>) -> Result<()> {
        self.log(AuditEvent::AuthenticationFailure {
            did: did.to_string(),
            reason: reason.to_string(),
            ip_address: ip.map(|s| s.to_string()),
            timestamp: Utc::now(),
        })
    }

    /// Log channel opened
    pub fn log_channel_opened(&self, channel_id: &str, party_a: &str, party_b: &str) -> Result<()> {
        self.log(AuditEvent::ChannelOpened {
            channel_id: channel_id.to_string(),
            party_a: party_a.to_string(),
            party_b: party_b.to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Log channel closed
    pub fn log_channel_closed(
        &self,
        channel_id: &str,
        final_state: ChannelState,
        balance_a: u64,
        balance_b: u64,
    ) -> Result<()> {
        self.log(AuditEvent::ChannelClosed {
            channel_id: channel_id.to_string(),
            final_state,
            final_balance_a: balance_a,
            final_balance_b: balance_b,
            timestamp: Utc::now(),
        })
    }

    /// Log security violation
    pub fn log_security_violation(
        &self,
        violation_type: &str,
        details: &str,
        severity: &str,
    ) -> Result<()> {
        self.log(AuditEvent::SecurityViolation {
            violation_type: violation_type.to_string(),
            details: details.to_string(),
            severity: severity.to_string(),
            timestamp: Utc::now(),
        })
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::with_logging()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::KeyGenerated {
            key_id: "key-123".to_string(),
            key_type: "ed25519".to_string(),
            timestamp: Utc::now(),
        };

        assert_eq!(event.event_type(), "key_generated");
        assert!(event.timestamp() <= Utc::now());
    }

    #[tokio::test]
    async fn test_memory_sink() {
        let sink = Arc::new(MemoryAuditSink::new(100));
        let logger = AuditLogger::new(sink.clone());

        logger.log_key_generated("key-1", "ed25519").unwrap();
        logger.log_key_rotated("key-1", 1, 2).unwrap();

        let events = sink.get_events().await;
        assert_eq!(events.len(), 2);

        let key_events = sink.get_events_by_type("key_generated").await;
        assert_eq!(key_events.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_sink_overflow() {
        let sink = Arc::new(MemoryAuditSink::new(5));
        let logger = AuditLogger::new(sink.clone());

        for i in 0..10 {
            logger
                .log_key_generated(&format!("key-{}", i), "ed25519")
                .unwrap();
        }

        let events = sink.get_events().await;
        assert_eq!(events.len(), 5);
        // Should have the last 5 events
        assert!(events.iter().any(|e| {
            if let AuditEvent::KeyGenerated { key_id, .. } = e {
                key_id == "key-9"
            } else {
                false
            }
        }));
    }
}
