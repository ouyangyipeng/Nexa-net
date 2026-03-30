//! gRPC Server

use crate::error::Result;

/// gRPC server
pub struct GrpcServer {
    /// Port
    port: u16,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(port: u16) -> Self {
        Self { port }
    }
    
    /// Start the server
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting gRPC server on port {}", self.port);
        Ok(())
    }
}