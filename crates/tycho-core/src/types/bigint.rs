//! BigInt conversion utilities.
//!
//! Provides utilities for working with arbitrary-precision unsigned integers.
//!
//! # Transport-Agnostic Design
//!
//! This module provides a `BigIntValue` type that can be serialized/deserialized
//! without depending on any specific transport (gRPC, JSON, etc.).
//!
//! Example: `"1000000000000000000"` represents 1e18 (1 ETH in wei)
//!
//! # Usage
//!
//! ```ignore
//! use tycho_core::types::bigint::{BigIntValue, from_string, to_string};
//!
//! // Parse from string
//! let value = from_string("1000000")?;
//!
//! // Convert to string
//! let s = to_string(&value);
//! assert_eq!(s, "1000000");
//!
//! // Use BigIntValue for serialization
//! let bi = BigIntValue::from_biguint(&value);
//! let recovered = bi.to_biguint()?;
//! ```
//!
//! # Error Handling
//!
//! Conversion from string to BigUint can fail if:
//! - The value contains non-numeric characters
//! - The value has invalid formatting

use num_bigint::BigUint;
use num_traits::Zero;
use std::str::FromStr;

use super::{ConversionError, ConversionResult};

/// A transport-agnostic BigInt representation.
///
/// Uses a decimal string internally, which can be easily converted to/from
/// proto, JSON, or any other format.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BigIntValue {
    /// Decimal string representation of the big integer.
    pub value: String,
}

impl BigIntValue {
    /// Create a new BigIntValue from a string.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Create a BigIntValue from a BigUint.
    pub fn from_biguint(value: &BigUint) -> Self {
        Self {
            value: value.to_string(),
        }
    }

    /// Convert to BigUint.
    pub fn to_biguint(&self) -> ConversionResult<BigUint> {
        to_biguint(self)
    }

    /// Create a zero BigIntValue.
    pub fn zero() -> Self {
        Self {
            value: "0".to_string(),
        }
    }
}

impl From<BigUint> for BigIntValue {
    fn from(value: BigUint) -> Self {
        Self::from_biguint(&value)
    }
}

impl From<u64> for BigIntValue {
    fn from(value: u64) -> Self {
        Self::new(value.to_string())
    }
}

impl From<u128> for BigIntValue {
    fn from(value: u128) -> Self {
        Self::new(value.to_string())
    }
}

// Type alias for backwards compatibility
pub type BigInt = BigIntValue;

/// Convert a `BigUint` to a `BigIntValue`.
///
/// This conversion is infallible as any `BigUint` can be represented as a decimal string.
///
/// # Arguments
///
/// * `value` - The BigUint value to convert
///
/// # Returns
///
/// A `BigIntValue` with the decimal string representation
///
/// # Example
///
/// ```ignore
/// let big_uint = BigUint::from(1_000_000u64);
/// let bi = from_biguint(&big_uint);
/// assert_eq!(bi.value, "1000000");
/// ```
pub fn from_biguint(value: &BigUint) -> BigIntValue {
    BigIntValue {
        value: value.to_string(),
    }
}

/// Convert a `BigIntValue` to a `BigUint`.
///
/// # Arguments
///
/// * `bi` - The BigIntValue to convert
///
/// # Returns
///
/// A `BigUint` if parsing succeeds, or a `ConversionError` if the value is invalid
///
/// # Errors
///
/// Returns `ConversionError::InvalidBigInt` if:
/// - The value contains non-numeric characters
///
/// # Example
///
/// ```ignore
/// let bi = BigIntValue { value: "1000000".to_string() };
/// let big_uint = to_biguint(&bi)?;
/// assert_eq!(big_uint, BigUint::from(1_000_000u64));
/// ```
pub fn to_biguint(bi: &BigIntValue) -> ConversionResult<BigUint> {
    if bi.value.is_empty() {
        return Ok(BigUint::zero());
    }

    BigUint::from_str(&bi.value).map_err(|_| ConversionError::InvalidBigInt(bi.value.clone()))
}

/// Convert an optional `BigIntValue` to a `BigUint`.
///
/// Returns zero if the optional is `None`.
///
/// # Arguments
///
/// * `bi` - The optional BigIntValue to convert
///
/// # Returns
///
/// A `BigUint` (zero if None) or a `ConversionError` if the value is invalid
pub fn to_biguint_optional(bi: Option<&BigIntValue>) -> ConversionResult<BigUint> {
    match bi {
        Some(b) => to_biguint(b),
        None => Ok(BigUint::zero()),
    }
}

/// Create a `BigIntValue` from a u64 value.
///
/// Convenience function for common small integer values.
///
/// # Arguments
///
/// * `value` - The u64 value to convert
///
/// # Returns
///
/// A `BigIntValue` with the decimal string representation
pub fn from_u64(value: u64) -> BigIntValue {
    BigIntValue {
        value: value.to_string(),
    }
}

/// Create a `BigIntValue` from a u128 value.
///
/// Convenience function for larger integer values that fit in u128.
///
/// # Arguments
///
/// * `value` - The u128 value to convert
///
/// # Returns
///
/// A `BigIntValue` with the decimal string representation
pub fn from_u128(value: u128) -> BigIntValue {
    BigIntValue {
        value: value.to_string(),
    }
}

/// Create a zero `BigIntValue`.
///
/// Convenience function for default/zero values.
pub fn zero() -> BigIntValue {
    BigIntValue {
        value: "0".to_string(),
    }
}

/// Parse a BigUint from a decimal string.
///
/// # Arguments
///
/// * `s` - The decimal string to parse
///
/// # Returns
///
/// A `BigUint` if parsing succeeds, or a `ConversionError` if invalid
pub fn from_string(s: &str) -> ConversionResult<BigUint> {
    if s.is_empty() {
        return Ok(BigUint::zero());
    }
    BigUint::from_str(s).map_err(|_| ConversionError::InvalidBigInt(s.to_string()))
}

/// Convert a BigUint to a decimal string.
pub fn to_string(value: &BigUint) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_biguint() {
        let value = BigUint::from(1_000_000u64);
        let proto = from_biguint(&value);
        assert_eq!(proto.value, "1000000");
    }

    #[test]
    fn test_from_biguint_zero() {
        let value = BigUint::zero();
        let proto = from_biguint(&value);
        assert_eq!(proto.value, "0");
    }

    #[test]
    fn test_from_biguint_large() {
        // 1e18 (1 ETH in wei)
        let value = BigUint::from(1_000_000_000_000_000_000u64);
        let proto = from_biguint(&value);
        assert_eq!(proto.value, "1000000000000000000");
    }

    #[test]
    fn test_to_biguint_valid() {
        let proto = BigInt {
            value: "1000000".to_string(),
        };
        let result = to_biguint(&proto);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::from(1_000_000u64));
    }

    #[test]
    fn test_to_biguint_zero() {
        let proto = BigInt {
            value: "0".to_string(),
        };
        let result = to_biguint(&proto);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::zero());
    }

    #[test]
    fn test_to_biguint_empty_string() {
        let proto = BigInt {
            value: "".to_string(),
        };
        let result = to_biguint(&proto);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::zero());
    }

    #[test]
    fn test_to_biguint_large() {
        let proto = BigInt {
            value: "1000000000000000000".to_string(),
        };
        let result = to_biguint(&proto);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::from(1_000_000_000_000_000_000u64));
    }

    #[test]
    fn test_to_biguint_invalid() {
        let proto = BigInt {
            value: "not_a_number".to_string(),
        };
        let result = to_biguint(&proto);
        assert!(result.is_err());
        match result {
            Err(ConversionError::InvalidBigInt(s)) => assert_eq!(s, "not_a_number"),
            _ => panic!("Expected InvalidBigInt error"),
        }
    }

    #[test]
    fn test_to_biguint_negative() {
        // BigUint doesn't support negative numbers
        let proto = BigInt {
            value: "-100".to_string(),
        };
        let result = to_biguint(&proto);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_biguint_with_whitespace() {
        let proto = BigInt {
            value: " 100 ".to_string(),
        };
        let result = to_biguint(&proto);
        // BigUint::from_str doesn't accept whitespace
        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = BigUint::from(123_456_789_012_345_678_901u128);
        let proto = from_biguint(&original);
        let recovered = to_biguint(&proto).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_to_biguint_optional_some() {
        let proto = BigInt {
            value: "42".to_string(),
        };
        let result = to_biguint_optional(Some(&proto));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::from(42u64));
    }

    #[test]
    fn test_to_biguint_optional_none() {
        let result = to_biguint_optional(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::zero());
    }

    #[test]
    fn test_from_u64() {
        let proto = from_u64(12345);
        assert_eq!(proto.value, "12345");
    }

    #[test]
    fn test_from_u128() {
        let proto = from_u128(u128::MAX);
        assert_eq!(proto.value, u128::MAX.to_string());
    }

    #[test]
    fn test_zero() {
        let proto = zero();
        assert_eq!(proto.value, "0");
    }
}
