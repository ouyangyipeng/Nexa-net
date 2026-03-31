//! REST API Server

use crate::error::Result;

/// REST API server
pub struct RestServer {
    /// Port
    port: u16,
}

impl RestServer {
    /// Create a new REST server
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Start the server
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting REST API on port {}", self.port);
        Ok(())
    }
}
