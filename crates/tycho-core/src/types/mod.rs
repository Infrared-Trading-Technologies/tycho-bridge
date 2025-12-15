//! Type definitions and conversions for Tycho Core.
//!
//! This module provides:
//! - Native request/response types for bridge operations
//! - Transport-agnostic type representations (BigIntValue, PriceInfo, TokenInfo, etc.)
//! - Conversion utilities for Tycho SDK types
//!
//! Note: Proto-specific conversions are handled in the tycho-grpc crate.

pub mod bigint;
pub mod component;
pub mod price;
pub mod request;
pub mod response;
pub mod token;

// Re-export native types
pub use bigint::{BigInt, BigIntValue};
pub use component::{Chain, ProtocolComponentInfo};
pub use price::{Price, PriceInfo};
pub use token::TokenInfo;

// Re-export commonly used conversion functions
pub use bigint::{from_biguint, from_string, to_biguint, to_string};
pub use component::{
    component_contains_tokens, component_key_info, component_matches_protocol, evm_chain_to_native,
    evm_chain_to_u64, format_bytes, parse_bytes,
};
pub use price::{
    from_f64, from_ratio, from_tycho_price, price_to_f64, to_f64, to_tycho_price, unity,
};
pub use token::{
    chain_id_to_tycho_chain, format_address, is_valid_address, normalize_address, parse_address,
    tycho_chain_to_chain_id,
};

/// Error type for type conversion failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConversionError {
    /// Failed to parse a BigInt string value.
    #[error("invalid BigInt value: {0}")]
    InvalidBigInt(String),

    /// Failed to parse a Price (missing or invalid numerator/denominator).
    #[error("invalid Price: {0}")]
    InvalidPrice(String),

    /// Failed to parse an address (invalid hex format).
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Unknown chain ID encountered.
    #[error("unknown chain ID: {0}")]
    UnknownChain(u64),

    /// Missing required field.
    #[error("missing required field: {0}")]
    MissingField(String),
}

/// Result type alias for conversion operations.
pub type ConversionResult<T> = Result<T, ConversionError>;
