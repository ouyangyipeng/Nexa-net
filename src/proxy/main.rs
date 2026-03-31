//! Nexa-Proxy Main Entry Point
//!
//! The Nexa-Proxy is a local sidecar daemon that handles network
//! communication for agents.

use nexa_net::error::Result;
use nexa_net::proxy::{ProxyConfig, ProxyServer};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Nexa-Proxy v{}", nexa_net::VERSION);

    // Load configuration
    let config = ProxyConfig::default();

    // Create and run server
    let mut server = ProxyServer::new(config);
    server.run().await?;

    tracing::info!("Nexa-Proxy stopped");
    Ok(())
}
