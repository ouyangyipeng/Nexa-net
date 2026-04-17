//! gRPC Server using Tonic
//!
//! Implements the gRPC service endpoints for Nexa-Proxy as defined in
//! PROTOCOL_SPEC.md and ARCHITECTURE.md.
//!
//! # Services
//!
//! - **Health**: gRPC health checking protocol (standard)
//! - Future: Identity, Discovery, Transport, Economy proto services

use crate::error::{Error, Result};
use crate::proxy::server::ProxyState;
use std::sync::Arc;
use tonic::transport::Server;

// ============================================================================
// gRPC Server
// ============================================================================

/// gRPC server wrapper that manages all Tonic services
pub struct GrpcServer {
    /// Bind address
    bind: String,
    /// Port
    port: u16,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(bind: &str, port: u16) -> Self {
        Self {
            bind: bind.to_string(),
            port,
        }
    }

    /// Start the gRPC server with standard health checking
    ///
    /// Uses `tonic_health::server::health_reporter()` which follows
    /// the gRPC Health Checking Protocol specification.
    /// The overall server health is set to `Serving` by default.
    pub async fn start(&self, _state: Arc<ProxyState>) -> Result<()> {
        let addr = format!("{}:{}", self.bind, self.port);
        tracing::info!("Starting gRPC server on {}", addr);

        let addr_parsed = addr
            .parse()
            .map_err(|e| Error::Internal(format!("Invalid gRPC address '{}': {}", addr, e)))?;

        // Create health reporter — the overall server status is already
        // set to Serving by default in HealthReporter::new()
        let (_health_reporter, health_service) = tonic_health::server::health_reporter();

        // Start Tonic server with health service
        // Future: add Identity, Discovery, Transport, Economy proto services
        Server::builder()
            .add_service(health_service)
            .serve(addr_parsed)
            .await
            .map_err(|e| Error::Internal(format!("gRPC server error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::server::ProxyState;

    #[test]
    fn test_grpc_server_creation() {
        let server = GrpcServer::new("127.0.0.1", 7071);
        assert_eq!(server.port, 7071);
        assert_eq!(server.bind, "127.0.0.1");
    }

    #[test]
    fn test_grpc_server_default_port() {
        let server = GrpcServer::new("127.0.0.1", 7071);
        assert_eq!(server.port, 7071);
    }
}
