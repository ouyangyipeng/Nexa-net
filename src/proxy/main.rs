//! Nexa-Proxy Main Entry Point
//!
//! Starts the local sidecar proxy daemon that provides:
//! - REST API server (Axum) for local Agent communication
//! - gRPC server (Tonic) for streaming/binary communication
//! - Identity, Discovery, Transport, and Economy services
//!
//! # Usage
//!
//! ```bash
//! # Start with default config
//! nexa-proxy
//!
//! # Start with custom config file
//! nexa-proxy --config /path/to/nexa-proxy.toml
//! ```

use nexa_net::api::grpc::GrpcServer;
use nexa_net::api::rest::RestServer;
use nexa_net::proxy::config::ProxyConfig;
use nexa_net::proxy::server::ProxyState;
use std::sync::Arc;

/// Main entry point for Nexa-Proxy
#[tokio::main]
async fn main() -> nexa_net::error::Result<()> {
    // Parse command-line arguments
    let config_path = parse_args();

    // Load configuration (with env overrides)
    let config = ProxyConfig::from_file_with_env(&config_path)?;
    config.validate()?;

    // Initialize tracing
    init_tracing(&config.log_level);

    tracing::info!("Nexa-Proxy v{} starting", env!("CARGO_PKG_VERSION"));
    tracing::info!(
        "Config: REST={}:{}, gRPC={}:{}",
        config.api_bind,
        config.api_port,
        config.api_bind,
        config.grpc_port
    );

    // Create shared proxy state
    let state = Arc::new(ProxyState::new());

    // Start REST and gRPC servers concurrently
    let rest_server = RestServer::new(&config.api_bind, config.api_port);
    let grpc_server = GrpcServer::new(&config.api_bind, config.grpc_port);

    let rest_state = state.clone();
    let grpc_state = state.clone();

    // Run both servers — if either fails, we exit
    let (rest_result, grpc_result) = tokio::join!(
        tokio::spawn(async move { rest_server.start(rest_state).await }),
        tokio::spawn(async move { grpc_server.start(grpc_state).await }),
    );

    // Check results
    match (rest_result, grpc_result) {
        (Ok(Ok(_)), Ok(Ok(_))) => {
            tracing::info!("Nexa-Proxy shutdown cleanly");
            Ok(())
        }
        (Ok(Err(e)), _) => {
            tracing::error!("REST server error: {}", e);
            Err(e)
        }
        (_, Ok(Err(e))) => {
            tracing::error!("gRPC server error: {}", e);
            Err(e)
        }
        (Err(e), _) => {
            tracing::error!("REST server task panicked: {}", e);
            Err(nexa_net::error::Error::Internal(format!(
                "REST task panic: {}",
                e
            )))
        }
        (_, Err(e)) => {
            tracing::error!("gRPC server task panicked: {}", e);
            Err(nexa_net::error::Error::Internal(format!(
                "gRPC task panic: {}",
                e
            )))
        }
    }
}

/// Parse command-line arguments
///
/// Currently supports:
/// - `--config <path>` or `-c <path>`: Specify config file path
/// - Default: `config/nexa-proxy.toml`
fn parse_args() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = "config/nexa-proxy.toml".to_string();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" || args[i] == "-c" {
            if i + 1 < args.len() {
                config_path = args[i + 1].clone();
                i += 2;
            } else {
                eprintln!("Error: --config requires a path argument");
                std::process::exit(1);
            }
        } else if args[i] == "--help" || args[i] == "-h" {
            println!("Nexa-Proxy - Decentralized M2M Communication Sidecar");
            println!();
            println!("Usage: nexa-proxy [OPTIONS]");
            println!();
            println!("Options:");
            println!(
                "  -c, --config <path>  Configuration file path (default: config/nexa-proxy.toml)"
            );
            println!("  -h, --help           Show this help message");
            println!("  -V, --version        Show version");
            std::process::exit(0);
        } else if args[i] == "--version" || args[i] == "-V" {
            println!("nexa-proxy v{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        } else {
            eprintln!("Unknown argument: {}", args[i]);
            std::process::exit(1);
        }
    }

    config_path
}

/// Initialize tracing subscriber based on log level
fn init_tracing(log_level: &str) {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    fmt().with_env_filter(filter).init();
}
