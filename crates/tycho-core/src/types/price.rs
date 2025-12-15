//! Price conversion utilities.
//!
//! Provides transport-agnostic types and utilities for working with prices.
//!
//! # Price Representation
//!
//! Prices are represented as rational fractions (numerator/denominator) to avoid
//! floating-point precision errors.
//!
//! # Tycho SDK Reference
//!
//! From tycho-simulation:
//! ```ignore
//! pub struct Price {
//!     pub numerator: BigUint,
//!     pub denominator: BigUint,
//! }
//! ```
//!
//! Source: https://docs.propellerheads.xyz/tycho/for-solvers/simulation#swap-to-price
//!
//! # Usage
//!
//! ```ignore
//! use tycho_core::types::price::{PriceInfo, from_ratio};
//!
//! // Create a price
//! let price = from_ratio(3000, 1);
//! let f = price_to_f64(&price);
//! assert_eq!(f, 3000.0);
//! ```

use num_bigint::BigUint;
use num_traits::{One, Zero};

use super::bigint::{from_biguint, to_biguint, BigIntValue};
use super::{ConversionError, ConversionResult};

// Import the Tycho SDK Price type
use tycho_simulation::tycho_common::simulation::protocol_sim::Price as TychoPrice;

/// Native Price representation - transport agnostic.
/// Uses rational fractions for exact math.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PriceInfo {
    /// Numerator as a BigInt string
    pub numerator: BigIntValue,
    /// Denominator as a BigInt string
    pub denominator: BigIntValue,
}

impl PriceInfo {
    /// Create a new PriceInfo from BigUint values.
    pub fn new(numerator: &BigUint, denominator: &BigUint) -> Self {
        Self {
            numerator: from_biguint(numerator),
            denominator: from_biguint(denominator),
        }
    }

    /// Create a PriceInfo from a Tycho SDK Price.
    pub fn from_tycho(price: &TychoPrice) -> Self {
        Self {
            numerator: from_biguint(&price.numerator),
            denominator: from_biguint(&price.denominator),
        }
    }

    /// Convert to a Tycho SDK Price.
    pub fn to_tycho(&self) -> ConversionResult<TychoPrice> {
        let numerator = to_biguint(&self.numerator)?;
        let denominator = to_biguint(&self.denominator)?;

        if denominator.is_zero() {
            return Err(ConversionError::InvalidPrice(
                "denominator cannot be zero".to_string(),
            ));
        }

        Ok(TychoPrice {
            numerator,
            denominator,
        })
    }

    /// Convert to f64 for display/comparison.
    pub fn to_f64(&self) -> f64 {
        match self.to_tycho() {
            Ok(price) => price_to_f64(&price),
            Err(_) => f64::NAN,
        }
    }

    /// Create a unity (1:1) price.
    pub fn unity() -> Self {
        Self {
            numerator: BigIntValue::new("1"),
            denominator: BigIntValue::new("1"),
        }
    }

    /// Create a price from u64 ratio.
    pub fn from_ratio(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator: BigIntValue::from(numerator),
            denominator: BigIntValue::from(denominator),
        }
    }
}

impl From<TychoPrice> for PriceInfo {
    fn from(price: TychoPrice) -> Self {
        Self::from_tycho(&price)
    }
}

impl From<&TychoPrice> for PriceInfo {
    fn from(price: &TychoPrice) -> Self {
        Self::from_tycho(price)
    }
}

impl Default for PriceInfo {
    fn default() -> Self {
        Self::unity()
    }
}

// Legacy type alias for backwards compatibility
pub type Price = PriceInfo;

/// Convert a Tycho SDK `Price` to a `PriceInfo`.
///
/// This conversion is infallible as any Tycho Price can be represented.
///
/// # Arguments
///
/// * `price` - The Tycho SDK price to convert
///
/// # Returns
///
/// A `PriceInfo` with numerator and denominator as BigInt strings
pub fn from_tycho_price(price: &TychoPrice) -> PriceInfo {
    PriceInfo::from_tycho(price)
}

/// Convert a `PriceInfo` to a Tycho SDK `Price`.
///
/// # Arguments
///
/// * `price_info` - The PriceInfo to convert
///
/// # Returns
///
/// A Tycho SDK `Price` if parsing succeeds, or a `ConversionError` if invalid
///
/// # Errors
///
/// Returns `ConversionError::InvalidPrice` if:
/// - Numerator or denominator is invalid
/// - Denominator is zero
pub fn to_tycho_price(price_info: &PriceInfo) -> ConversionResult<TychoPrice> {
    price_info.to_tycho()
}

/// Create a `PriceInfo` from a floating-point value.
///
/// Converts a float to a rational approximation with high precision.
/// Uses a fixed denominator of 10^18 for consistency with token decimals.
///
/// # Arguments
///
/// * `value` - The floating-point price value
///
/// # Returns
///
/// A `PriceInfo` representing the approximate rational value
///
/// # Note
///
/// This function may introduce small rounding errors. For exact math,
/// prefer constructing prices directly as rationals.
pub fn from_f64(value: f64) -> PriceInfo {
    // Use 10^18 as standard denominator for precision
    let denominator = BigUint::from(10u64.pow(18));
    let numerator_f64 = value * 1e18;
    let numerator = BigUint::from(numerator_f64 as u128);

    PriceInfo::new(&numerator, &denominator)
}

/// Convert a `PriceInfo` to a floating-point value.
///
/// # Arguments
///
/// * `price_info` - The PriceInfo to convert
///
/// # Returns
///
/// The price as a f64, or an error if the price is invalid
///
/// # Note
///
/// May lose precision for very large or very small prices.
pub fn to_f64(price_info: &PriceInfo) -> ConversionResult<f64> {
    let price = price_info.to_tycho()?;
    Ok(price_to_f64(&price))
}

/// Convert a Tycho SDK `Price` to a floating-point value.
///
/// # Arguments
///
/// * `price` - The Tycho SDK price
///
/// # Returns
///
/// The price as a f64
///
/// # Note
///
/// May lose precision for very large or very small prices.
pub fn price_to_f64(price: &TychoPrice) -> f64 {
    if price.denominator.is_zero() {
        return f64::INFINITY;
    }

    // Convert to f64 by dividing
    let num_str = price.numerator.to_string();
    let den_str = price.denominator.to_string();

    let num: f64 = num_str.parse().unwrap_or(f64::NAN);
    let den: f64 = den_str.parse().unwrap_or(1.0);

    num / den
}

/// Create a Tycho SDK `Price` representing 1:1 (unity).
pub fn unity() -> TychoPrice {
    TychoPrice {
        numerator: BigUint::one(),
        denominator: BigUint::one(),
    }
}

/// Create a `PriceInfo` representing 1:1 (unity).
pub fn price_info_unity() -> PriceInfo {
    PriceInfo::unity()
}

/// Create a Tycho SDK `Price` from simple integer ratio.
///
/// # Arguments
///
/// * `numerator` - The numerator as u64
/// * `denominator` - The denominator as u64
///
/// # Returns
///
/// A Tycho SDK `Price`
pub fn from_ratio(numerator: u64, denominator: u64) -> TychoPrice {
    TychoPrice {
        numerator: BigUint::from(numerator),
        denominator: BigUint::from(denominator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_tycho_price() {
        let tycho = TychoPrice {
            numerator: BigUint::from(3000u64),
            denominator: BigUint::from(1u64),
        };
        let price_info = from_tycho_price(&tycho);

        assert_eq!(price_info.numerator.value, "3000");
        assert_eq!(price_info.denominator.value, "1");
    }

    #[test]
    fn test_to_tycho_price_valid() {
        let price_info = PriceInfo {
            numerator: BigIntValue::new("3000"),
            denominator: BigIntValue::new("1"),
        };
        let result = to_tycho_price(&price_info);
        assert!(result.is_ok());
        let tycho = result.unwrap();
        assert_eq!(tycho.numerator, BigUint::from(3000u64));
        assert_eq!(tycho.denominator, BigUint::from(1u64));
    }

    #[test]
    fn test_to_tycho_price_zero_denominator() {
        let price_info = PriceInfo {
            numerator: BigIntValue::new("3000"),
            denominator: BigIntValue::new("0"),
        };
        let result = to_tycho_price(&price_info);
        assert!(result.is_err());
        match result {
            Err(ConversionError::InvalidPrice(msg)) => {
                assert!(msg.contains("zero"));
            }
            _ => panic!("Expected InvalidPrice error"),
        }
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = TychoPrice {
            numerator: BigUint::from(123_456_789u64),
            denominator: BigUint::from(987_654_321u64),
        };
        let price_info = from_tycho_price(&original);
        let recovered = to_tycho_price(&price_info).unwrap();

        assert_eq!(original.numerator, recovered.numerator);
        assert_eq!(original.denominator, recovered.denominator);
    }

    #[test]
    fn test_from_f64() {
        let price_info = from_f64(1.5);
        let f = to_f64(&price_info).unwrap();
        // Allow for floating point error
        assert!((f - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_price_to_f64() {
        let price = TychoPrice {
            numerator: BigUint::from(3u64),
            denominator: BigUint::from(2u64),
        };
        let f = price_to_f64(&price);
        assert!((f - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_price_to_f64_zero_denominator() {
        let price = TychoPrice {
            numerator: BigUint::from(1u64),
            denominator: BigUint::zero(),
        };
        let f = price_to_f64(&price);
        assert!(f.is_infinite());
    }

    #[test]
    fn test_unity() {
        let price = unity();
        assert_eq!(price.numerator, BigUint::one());
        assert_eq!(price.denominator, BigUint::one());
    }

    #[test]
    fn test_price_info_unity() {
        let price_info = price_info_unity();
        assert_eq!(price_info.numerator.value, "1");
        assert_eq!(price_info.denominator.value, "1");
    }

    #[test]
    fn test_from_ratio() {
        let price = from_ratio(3, 4);
        let f = price_to_f64(&price);
        assert!((f - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_price_info_from_ratio() {
        let price_info = PriceInfo::from_ratio(3, 4);
        let f = price_info.to_f64();
        assert!((f - 0.75).abs() < 1e-10);
    }
}
