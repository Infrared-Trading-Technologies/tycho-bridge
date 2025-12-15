//! Core helper functions for bridge operations.
//!
//! These are transport-agnostic utility functions used by the handlers.
//! Transport layers (gRPC, FFI) may wrap these with their own error types.

use num_bigint::BigUint;
use num_traits::Zero;

use crate::domain::Token;
use crate::error::{BridgeError, BridgeResult};

// ─────────────────────────────────────────────────────────────────────────────
// Pagination Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Default pagination limit for list queries.
pub const DEFAULT_PAGE_SIZE: u32 = 100;

/// Maximum pagination limit to prevent resource exhaustion.
pub const MAX_PAGE_SIZE: u32 = 1000;

// ─────────────────────────────────────────────────────────────────────────────
// Pagination Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Apply pagination limits with bounds checking.
///
/// Returns (page_size, offset) tuple.
/// - page_size is bounded between 1 and MAX_PAGE_SIZE
/// - offset is calculated from the page number
pub fn apply_pagination(page: u32, page_size: u32) -> (usize, usize) {
    let page_size = if page_size == 0 {
        DEFAULT_PAGE_SIZE as usize
    } else {
        page_size.min(MAX_PAGE_SIZE) as usize
    };
    let page = page.max(1) as usize;
    let offset = (page - 1) * page_size;
    (page_size, offset)
}

// ─────────────────────────────────────────────────────────────────────────────
// Address Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Normalize an address string to lowercase without 0x prefix.
///
/// This is used for consistent address comparisons.
pub fn normalize_address(addr: &str) -> BridgeResult<String> {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return Err(BridgeError::invalid_argument("empty address"));
    }
    let normalized = crate::types::normalize_address(trimmed);
    if normalized.is_empty() || !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(BridgeError::invalid_argument(format!(
            "invalid hex: {}",
            addr
        )));
    }
    Ok(normalized)
}

/// Parse a hex address string to Tycho `Bytes` type.
pub fn parse_address(addr: &str) -> BridgeResult<tycho_simulation::tycho_common::Bytes> {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return Err(BridgeError::invalid_argument("empty address"));
    }
    crate::types::parse_address(trimmed).map_err(|e| BridgeError::invalid_argument(e.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Token Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Find a token in the component's token list by address.
///
/// Compares addresses case-insensitively after normalizing.
pub fn find_token_in_component(tokens: &[Token], target_addr: &str) -> Option<Token> {
    tokens
        .iter()
        .find(|t| {
            let token_addr = hex::encode(&t.address).to_lowercase();
            token_addr == target_addr || token_addr.ends_with(target_addr)
        })
        .cloned()
}

// ─────────────────────────────────────────────────────────────────────────────
// Price Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Calculate execution price as a float.
///
/// Handles decimal adjustment between input and output tokens.
pub fn calculate_execution_price(
    amount_in: &BigUint,
    amount_out: &BigUint,
    decimals_in: u32,
    decimals_out: u32,
) -> f64 {
    if amount_in.is_zero() {
        return 0.0;
    }

    let in_f64 = amount_in.to_string().parse::<f64>().unwrap_or(0.0);
    let out_f64 = amount_out.to_string().parse::<f64>().unwrap_or(0.0);

    if in_f64 == 0.0 {
        return 0.0;
    }

    let decimal_adjustment = 10f64.powi(decimals_in as i32 - decimals_out as i32);
    (out_f64 / in_f64) * decimal_adjustment
}

/// Calculate price impact in basis points.
///
/// Returns 0 if either price is zero or negative.
pub fn calculate_price_impact_bps(spot_before: f64, effective_price: f64) -> f64 {
    if spot_before > 0.0 && effective_price > 0.0 {
        ((spot_before - effective_price) / spot_before * 10000.0).max(0.0)
    } else {
        0.0
    }
}

/// Convert f64 price to a rational approximation.
///
/// Uses the existing price module's conversion.
pub fn float_to_rational(price: f64) -> crate::types::PriceInfo {
    crate::types::from_f64(price)
}

// ─────────────────────────────────────────────────────────────────────────────
// Chain Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert chain ID to Tycho SDK Chain.
///
/// Used for tycho-execution encoding.
pub fn chain_id_to_tycho_chain(
    chain_id: u64,
) -> BridgeResult<tycho_simulation::tycho_common::models::Chain> {
    crate::types::chain_id_to_tycho_chain(chain_id)
        .map_err(|_| BridgeError::ChainNotSupported { chain_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pagination() {
        let (size, offset) = apply_pagination(1, 10);
        assert_eq!(size, 10);
        assert_eq!(offset, 0);

        let (size, offset) = apply_pagination(2, 10);
        assert_eq!(size, 10);
        assert_eq!(offset, 10);

        let (size, offset) = apply_pagination(0, 0);
        assert_eq!(size, DEFAULT_PAGE_SIZE as usize);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_normalize_address() {
        assert_eq!(normalize_address("0xABC123").unwrap(), "abc123");
        assert_eq!(normalize_address("  0xDEF456  ").unwrap(), "def456");
        assert!(normalize_address("0xGGG").is_err());
        assert!(normalize_address("").is_err());
    }

    #[test]
    fn test_calculate_execution_price() {
        // Test basic price calculation: 1000 USDC (6 decimals) for 1 ETH (18 decimals)
        let amount_in = BigUint::from(1_000_000_000_000_000_000u64); // 1 ETH (1e18)
        let amount_out = BigUint::from(1_000_000_000u64); // 1000 USDC (1e9 = 1000 * 1e6)
        let price = calculate_execution_price(&amount_in, &amount_out, 18, 6);
        // Expected: (1e9 / 1e18) * 10^(18-6) = 1e9 / 1e18 * 1e12 = 1e-9 * 1e12 = 1000
        assert!((price - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_execution_price_zero_input() {
        let amount_in = BigUint::from(0u64);
        let amount_out = BigUint::from(1000u64);
        let price = calculate_execution_price(&amount_in, &amount_out, 18, 6);
        assert_eq!(price, 0.0);
    }

    #[test]
    fn test_calculate_price_impact_bps() {
        let impact = calculate_price_impact_bps(100.0, 99.0);
        assert!((impact - 100.0).abs() < 0.01); // 1% = 100 bps

        let impact = calculate_price_impact_bps(100.0, 100.0);
        assert_eq!(impact, 0.0);

        let impact = calculate_price_impact_bps(0.0, 99.0);
        assert_eq!(impact, 0.0);
    }
}
