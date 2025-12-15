//! Configuration module for Tycho Bridge.
//!
//! Follows 12-Factor App principles:
//! - All configuration loaded from environment variables
//! - Strict struct validation ensures fail-fast behavior
//! - No hardcoded defaults for sensitive values

use serde::Deserialize;
use std::fmt;

/// Environment-based configuration for Tycho Bridge.
///
/// All fields are populated from environment variables following the 12-Factor App
/// methodology. Required fields will cause startup failure if not provided.
///
/// Environment variable names are derived from field names:
/// - `tycho_url` -> `TYCHO_URL`
/// - `tycho_api_key` -> `TYCHO_API_KEY`
/// - etc.
#[derive(Clone, Deserialize)]
pub struct Config {
    /// WebSocket URL for Tycho indexer (e.g., "wss://tycho-beta.propellerheads.xyz")
    pub tycho_url: String,

    /// API key for authenticating with Tycho service
    pub tycho_api_key: String,

    /// gRPC server listen address (default: "0.0.0.0")
    #[serde(default = "default_grpc_host")]
    pub grpc_host: String,

    /// gRPC server port (default: 50051)
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,

    /// Prometheus metrics port (default: 9000)
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Comma-separated list of chain IDs to sync (e.g., "1,8453,130")
    #[serde(default = "default_chain_ids")]
    pub chain_ids: String,

    /// Log level filter (e.g., "info", "debug", "tycho_bridge=debug,info")
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_grpc_host() -> String {
    "0.0.0.0".to_string()
}

fn default_grpc_port() -> u16 {
    50051
}

fn default_metrics_port() -> u16 {
    9000
}

fn default_chain_ids() -> String {
    "1".to_string() // Ethereum mainnet by default
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::from_env::<Config>().map_err(ConfigError::Parse)
    }

    /// Parse chain IDs from the comma-separated string.
    ///
    /// # Errors
    ///
    /// Returns an error if any chain ID cannot be parsed as u64.
    pub fn parsed_chain_ids(&self) -> Result<Vec<u64>, ConfigError> {
        self.chain_ids
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<u64>()
                    .map_err(|_| ConfigError::InvalidChainId(s.to_string()))
            })
            .collect()
    }

    /// Get the full gRPC bind address.
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.grpc_host, self.grpc_port)
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to parse environment variables into config struct.
    #[error("failed to parse configuration from environment: {0}")]
    Parse(#[from] envy::Error),

    /// Invalid chain ID format.
    #[error("invalid chain ID format: '{0}' is not a valid u64")]
    InvalidChainId(String),
}

// Implement Debug manually to avoid exposing sensitive data in logs
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("tycho_url", &self.tycho_url)
            .field("tycho_api_key", &"[REDACTED]")
            .field("grpc_host", &self.grpc_host)
            .field("grpc_port", &self.grpc_port)
            .field("metrics_port", &self.metrics_port)
            .field("chain_ids", &self.chain_ids)
            .field("log_level", &self.log_level)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_chain_ids() {
        let config = Config {
            tycho_url: "wss://test".to_string(),
            tycho_api_key: "key".to_string(),
            grpc_host: "0.0.0.0".to_string(),
            grpc_port: 50051,
            metrics_port: 9000,
            chain_ids: "1, 8453, 130".to_string(),
            log_level: "info".to_string(),
        };

        let ids = config.parsed_chain_ids().unwrap();
        assert_eq!(ids, vec![1, 8453, 130]);
    }

    #[test]
    fn test_parsed_chain_ids_invalid() {
        let config = Config {
            tycho_url: "wss://test".to_string(),
            tycho_api_key: "key".to_string(),
            grpc_host: "0.0.0.0".to_string(),
            grpc_port: 50051,
            metrics_port: 9000,
            chain_ids: "1, invalid, 130".to_string(),
            log_level: "info".to_string(),
        };

        let result = config.parsed_chain_ids();
        assert!(result.is_err());
    }

    #[test]
    fn test_grpc_addr() {
        let config = Config {
            tycho_url: "wss://test".to_string(),
            tycho_api_key: "key".to_string(),
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 9090,
            metrics_port: 9000,
            chain_ids: "1".to_string(),
            log_level: "info".to_string(),
        };

        assert_eq!(config.grpc_addr(), "127.0.0.1:9090");
    }

    #[test]
    fn test_debug_redacts_api_key() {
        let config = Config {
            tycho_url: "wss://test".to_string(),
            tycho_api_key: "super-secret-key".to_string(),
            grpc_host: "0.0.0.0".to_string(),
            grpc_port: 50051,
            metrics_port: 9000,
            chain_ids: "1".to_string(),
            log_level: "info".to_string(),
        };

        let debug_output = format!("{:?}", config);
        assert!(!debug_output.contains("super-secret-key"));
        assert!(debug_output.contains("[REDACTED]"));
    }
}
