//! Bridge error types.
//!
//! These are transport-agnostic errors that can be converted to
//! gRPC Status, FFI error codes, or HTTP status codes.

use thiserror::Error;

/// Core error type for all bridge operations.
#[derive(Error, Debug, Clone)]
pub enum BridgeError {
    /// Component/pool not found in cache.
    #[error("Component not found: {component_id}")]
    ComponentNotFound { component_id: String },

    /// Token not found in component or cache.
    #[error("Token not found: {token}")]
    TokenNotFound { token: String },

    /// Invalid argument provided to operation.
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    /// Simulation failed.
    #[error("Simulation failed: {message}")]
    SimulationError { message: String },

    /// State not available (not synced, etc.).
    #[error("State not available: {message}")]
    StateUnavailable { message: String },

    /// Chain not supported by this bridge instance.
    #[error("Chain not supported: {chain_id}")]
    ChainNotSupported { chain_id: u64 },

    /// Configuration error.
    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    /// Connection error.
    #[error("Connection error: {message}")]
    ConnectionError { message: String },

    /// Internal error.
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl BridgeError {
    /// Get numeric error code for FFI.
    pub fn code(&self) -> i32 {
        match self {
            BridgeError::ComponentNotFound { .. } => 1,
            BridgeError::TokenNotFound { .. } => 2,
            BridgeError::InvalidArgument { .. } => 3,
            BridgeError::SimulationError { .. } => 4,
            BridgeError::StateUnavailable { .. } => 5,
            BridgeError::ChainNotSupported { .. } => 6,
            BridgeError::ConfigError { .. } => 7,
            BridgeError::ConnectionError { .. } => 8,
            BridgeError::Internal { .. } => 99,
        }
    }

    /// Create an invalid argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        BridgeError::InvalidArgument {
            message: message.into(),
        }
    }

    /// Create a component not found error.
    pub fn component_not_found(component_id: impl Into<String>) -> Self {
        BridgeError::ComponentNotFound {
            component_id: component_id.into(),
        }
    }

    /// Create a token not found error.
    pub fn token_not_found(token: impl Into<String>) -> Self {
        BridgeError::TokenNotFound {
            token: token.into(),
        }
    }

    /// Create a simulation error.
    pub fn simulation_error(message: impl Into<String>) -> Self {
        BridgeError::SimulationError {
            message: message.into(),
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        BridgeError::Internal {
            message: message.into(),
        }
    }
}

/// Result type alias for bridge operations.
pub type BridgeResult<T> = Result<T, BridgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_are_unique() {
        // Each error variant should have a distinct code
        let errors = [
            BridgeError::ComponentNotFound { component_id: "x".into() },
            BridgeError::TokenNotFound { token: "x".into() },
            BridgeError::InvalidArgument { message: "x".into() },
            BridgeError::SimulationError { message: "x".into() },
            BridgeError::StateUnavailable { message: "x".into() },
            BridgeError::ChainNotSupported { chain_id: 1 },
            BridgeError::ConfigError { message: "x".into() },
            BridgeError::ConnectionError { message: "x".into() },
            BridgeError::Internal { message: "x".into() },
        ];

        let codes: Vec<i32> = errors.iter().map(|e| e.code()).collect();
        let unique: std::collections::HashSet<i32> = codes.iter().cloned().collect();
        assert_eq!(codes.len(), unique.len(), "Error codes must be unique");
    }

    #[test]
    fn test_error_codes_match_expected() {
        assert_eq!(BridgeError::component_not_found("x").code(), 1);
        assert_eq!(BridgeError::token_not_found("x").code(), 2);
        assert_eq!(BridgeError::invalid_argument("x").code(), 3);
        assert_eq!(BridgeError::simulation_error("x").code(), 4);
        assert_eq!(BridgeError::StateUnavailable { message: "x".into() }.code(), 5);
        assert_eq!(BridgeError::ChainNotSupported { chain_id: 1 }.code(), 6);
        assert_eq!(BridgeError::ConfigError { message: "x".into() }.code(), 7);
        assert_eq!(BridgeError::ConnectionError { message: "x".into() }.code(), 8);
        assert_eq!(BridgeError::internal("x").code(), 99);
    }

    #[test]
    fn test_error_display_messages() {
        let err = BridgeError::component_not_found("pool_123");
        assert_eq!(err.to_string(), "Component not found: pool_123");

        let err = BridgeError::token_not_found("0xabc");
        assert_eq!(err.to_string(), "Token not found: 0xabc");

        let err = BridgeError::ChainNotSupported { chain_id: 42 };
        assert_eq!(err.to_string(), "Chain not supported: 42");
    }

    #[test]
    fn test_constructor_helpers() {
        // Test that constructors accept various string types
        let _ = BridgeError::invalid_argument("static str");
        let _ = BridgeError::invalid_argument(String::from("String"));
        let _ = BridgeError::component_not_found("id");
        let _ = BridgeError::token_not_found("token");
        let _ = BridgeError::simulation_error("failed");
        let _ = BridgeError::internal("oops");
    }

    #[test]
    fn test_error_is_clone() {
        let err = BridgeError::internal("test");
        let cloned = err.clone();
        assert_eq!(err.code(), cloned.code());
    }
}
