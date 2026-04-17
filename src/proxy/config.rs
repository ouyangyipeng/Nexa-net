//! Proxy Configuration
//!
//! Loads configuration from TOML files, environment variables,
//! or defaults. Supports hot-reload via file watch (future).

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// API bind address
    pub api_bind: String,
    /// API port (REST)
    pub api_port: u16,
    /// gRPC port
    pub grpc_port: u16,
    /// Supernode addresses
    pub supernodes: Vec<String>,
    /// Default timeout ms
    pub default_timeout_ms: u64,
    /// Default budget in NEXA tokens
    pub default_budget: u64,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            api_bind: "127.0.0.1".to_string(),
            api_port: 7070,
            grpc_port: 7071,
            supernodes: vec![],
            default_timeout_ms: 30000,
            default_budget: 100,
            log_level: "info".to_string(),
        }
    }
}

impl ProxyConfig {
    /// Load configuration from a TOML file
    ///
    /// Falls back to defaults for missing fields.
    /// Returns an error only if the file cannot be read or parsed.
    pub fn from_file(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            tracing::warn!("Config file '{}' not found, using defaults", path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::Error::Config(format!("Failed to read config file '{}': {}", path, e))
        })?;

        let config: ProxyConfig = toml::from_str(&content).map_err(|e| {
            crate::error::Error::Config(format!("Failed to parse config file '{}': {}", path, e))
        })?;

        tracing::info!("Loaded config from '{}'", path);
        Ok(config)
    }

    /// Load configuration with environment variable overrides
    ///
    /// Environment variables take precedence over file values:
    /// - `NEXA_API_BIND` → api_bind
    /// - `NEXA_API_PORT` → api_port
    /// - `NEXA_GRPC_PORT` → grpc_port
    /// - `NEXA_LOG_LEVEL` → log_level
    /// - `NEXA_DEFAULT_BUDGET` → default_budget
    /// - `NEXA_TIMEOUT_MS` → default_timeout_ms
    pub fn from_file_with_env(path: &str) -> Result<Self> {
        let mut config = Self::from_file(path)?;

        // Apply environment variable overrides
        if let Ok(val) = std::env::var("NEXA_API_BIND") {
            config.api_bind = val;
        }
        if let Ok(val) = std::env::var("NEXA_API_PORT") {
            config.api_port = val.parse().unwrap_or(config.api_port);
        }
        if let Ok(val) = std::env::var("NEXA_GRPC_PORT") {
            config.grpc_port = val.parse().unwrap_or(config.grpc_port);
        }
        if let Ok(val) = std::env::var("NEXA_LOG_LEVEL") {
            config.log_level = val;
        }
        if let Ok(val) = std::env::var("NEXA_DEFAULT_BUDGET") {
            config.default_budget = val.parse().unwrap_or(config.default_budget);
        }
        if let Ok(val) = std::env::var("NEXA_TIMEOUT_MS") {
            config.default_timeout_ms = val.parse().unwrap_or(config.default_timeout_ms);
        }
        if let Ok(val) = std::env::var("NEXA_SUPERNODES") {
            config.supernodes = val.split(',').map(|s| s.trim().to_string()).collect();
        }

        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.api_port == 0 {
            return Err(crate::error::Error::Config(
                "api_port cannot be 0".to_string(),
            ));
        }
        if self.grpc_port == 0 {
            return Err(crate::error::Error::Config(
                "grpc_port cannot be 0".to_string(),
            ));
        }
        if self.api_port == self.grpc_port {
            return Err(crate::error::Error::Config(
                "api_port and grpc_port must be different".to_string(),
            ));
        }
        if self.default_budget == 0 {
            return Err(crate::error::Error::Config(
                "default_budget cannot be 0".to_string(),
            ));
        }
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            return Err(crate::error::Error::Config(format!(
                "Invalid log_level '{}', must be one of: {}",
                self.log_level,
                valid_levels.join(", ")
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert_eq!(config.api_bind, "127.0.0.1");
        assert_eq!(config.api_port, 7070);
        assert_eq!(config.grpc_port, 7071);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.default_budget, 100);
    }

    #[test]
    fn test_config_from_file_not_found() {
        let config = ProxyConfig::from_file("/nonexistent/path.toml").unwrap();
        // Should fall back to defaults
        assert_eq!(config.api_port, 7070);
    }

    #[test]
    fn test_config_from_file_valid() {
        let mut tmp = NamedTempFile::new().unwrap();
        let content = r#"
api_bind = "0.0.0.0"
api_port = 8080
grpc_port = 9090
supernodes = ["did:nexa:supernode1", "did:nexa:supernode2"]
default_timeout_ms = 60000
default_budget = 500
log_level = "debug"
"#;
        std::fs::write(tmp.path(), content).unwrap();

        let config = ProxyConfig::from_file(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(config.api_bind, "0.0.0.0");
        assert_eq!(config.api_port, 8080);
        assert_eq!(config.grpc_port, 9090);
        assert_eq!(config.supernodes.len(), 2);
        assert_eq!(config.default_timeout_ms, 60000);
        assert_eq!(config.default_budget, 500);
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_config_validation() {
        let config = ProxyConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_port() {
        let mut config = ProxyConfig::default();
        config.api_port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_same_ports() {
        let mut config = ProxyConfig::default();
        config.api_port = 7070;
        config.grpc_port = 7070;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_log_level() {
        let mut config = ProxyConfig::default();
        config.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_env_override() {
        std::env::set_var("NEXA_API_PORT", "9999");
        std::env::set_var("NEXA_LOG_LEVEL", "debug");

        let config = ProxyConfig::from_file_with_env("/nonexistent/path.toml").unwrap();
        assert_eq!(config.api_port, 9999);
        assert_eq!(config.log_level, "debug");

        std::env::remove_var("NEXA_API_PORT");
        std::env::remove_var("NEXA_LOG_LEVEL");
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let config = ProxyConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: ProxyConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.api_bind, config.api_bind);
        assert_eq!(parsed.api_port, config.api_port);
        assert_eq!(parsed.grpc_port, config.grpc_port);
    }
}
