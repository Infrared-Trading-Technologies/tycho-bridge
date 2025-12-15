//! Token conversion utilities.
//!
//! Provides transport-agnostic types and utilities for working with tokens.
//!
//! # Tycho SDK Token
//!
//! From tycho-common:
//! ```ignore
//! pub struct Token {
//!     pub address: Bytes,
//!     pub symbol: String,
//!     pub decimals: u32,
//!     pub tax: TransferTax,
//!     pub gas: Vec<Option<TransferCost>>,
//!     pub chain: Chain,
//!     pub quality: u32,
//! }
//! ```
//!
//! # Quality Scores
//!
//! | Score | Meaning |
//! |-------|---------|
//! | 100 | Normal token |
//! | 75 | Rebase token |
//! | 50 | Fee token |
//! | 10 | Token analysis failed at creation |
//! | 5-9 | Token analysis failed on cronjob |
//! | 0 | Failed to extract decimals onchain |
//!
//! Source: https://docs.propellerheads.xyz/tycho/for-solvers/indexer/clients

use std::str::FromStr;

use tycho_simulation::tycho_common::Bytes;

use super::component::Chain;
use super::{ConversionError, ConversionResult};

// Import the Tycho SDK Token type
use tycho_simulation::tycho_common::models::token::Token as TychoToken;

// Import Chain type for token construction
use tycho_simulation::tycho_common::models::Chain as TychoChain;

/// Native Token representation - transport agnostic.
/// Can be serialized to JSON, FFI, or converted to proto.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenInfo {
    /// Token address (hex string with 0x prefix)
    pub address: String,
    /// Token decimals (e.g., 18 for ETH, 6 for USDC)
    pub decimals: u32,
    /// Token symbol (e.g., "WETH", "USDC")
    pub symbol: String,
    /// Quality score (0-100)
    pub quality: u32,
    /// Chain (optional, for context)
    #[serde(default)]
    pub chain: Chain,
}

impl TokenInfo {
    /// Create a new TokenInfo.
    pub fn new(address: impl Into<String>, symbol: impl Into<String>, decimals: u32) -> Self {
        Self {
            address: address.into(),
            decimals,
            symbol: symbol.into(),
            quality: 100,
            chain: Chain::Ethereum,
        }
    }

    /// Create a TokenInfo from a Tycho SDK Token.
    pub fn from_tycho(token: &TychoToken) -> Self {
        Self {
            address: format!("0x{}", hex::encode(&token.address)),
            decimals: token.decimals,
            symbol: token.symbol.clone(),
            quality: token.quality,
            chain: Chain::from_tycho(&token.chain),
        }
    }

    /// Convert to a Tycho SDK Token.
    pub fn to_tycho(&self) -> ConversionResult<TychoToken> {
        let address = parse_address(&self.address)?;
        let chain = self.chain.to_tycho()?;

        Ok(TychoToken {
            address,
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            tax: 0,
            gas: vec![],
            chain,
            quality: self.quality,
        })
    }

    /// Convert to a Tycho SDK Token with a specific chain.
    pub fn to_tycho_with_chain(&self, chain: TychoChain) -> ConversionResult<TychoToken> {
        let address = parse_address(&self.address)?;

        Ok(TychoToken {
            address,
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            tax: 0,
            gas: vec![],
            chain,
            quality: self.quality,
        })
    }
}

impl From<TychoToken> for TokenInfo {
    fn from(token: TychoToken) -> Self {
        Self::from_tycho(&token)
    }
}

impl From<&TychoToken> for TokenInfo {
    fn from(token: &TychoToken) -> Self {
        Self::from_tycho(token)
    }
}

/// Parse a hex address string to Tycho Bytes.
///
/// Handles both `0x` prefixed and non-prefixed addresses.
///
/// # Arguments
///
/// * `address` - The hex address string
///
/// # Returns
///
/// A `Bytes` value if parsing succeeds, or a `ConversionError` if invalid
pub fn parse_address(address: &str) -> ConversionResult<Bytes> {
    let normalized = normalize_address(address);

    if normalized.is_empty() {
        return Err(ConversionError::InvalidAddress(address.to_string()));
    }

    Bytes::from_str(&normalized).map_err(|_| ConversionError::InvalidAddress(address.to_string()))
}

/// Normalize an address string to lowercase without 0x prefix.
///
/// # Arguments
///
/// * `address` - The hex address string
///
/// # Returns
///
/// The normalized address string
pub fn normalize_address(address: &str) -> String {
    address
        .strip_prefix("0x")
        .or_else(|| address.strip_prefix("0X"))
        .unwrap_or(address)
        .to_lowercase()
}

/// Format a Bytes address as a checksummed hex string with 0x prefix.
///
/// # Arguments
///
/// * `address` - The Bytes address
///
/// # Returns
///
/// A hex string with 0x prefix
pub fn format_address(address: &Bytes) -> String {
    format!("0x{}", hex::encode(address))
}

/// Check if an address string is valid.
///
/// # Arguments
///
/// * `address` - The hex address string
///
/// # Returns
///
/// True if the address is valid hex
pub fn is_valid_address(address: &str) -> bool {
    parse_address(address).is_ok()
}

/// Convert a chain ID to a Tycho Chain enum.
///
/// # Arguments
///
/// * `chain_id` - The numeric chain ID
///
/// # Returns
///
/// The corresponding `TychoChain`, or an error if unknown
pub fn chain_id_to_tycho_chain(chain_id: u64) -> ConversionResult<TychoChain> {
    match chain_id {
        1 => Ok(TychoChain::Ethereum),
        8453 => Ok(TychoChain::Base),
        130 => Ok(TychoChain::Unichain),
        42161 => Ok(TychoChain::Arbitrum),
        324 => Ok(TychoChain::ZkSync),
        _ => Err(ConversionError::UnknownChain(chain_id)),
    }
}

/// Convert a Tycho Chain enum to a chain ID.
///
/// # Arguments
///
/// * `chain` - The Tycho Chain enum
///
/// # Returns
///
/// The numeric chain ID
pub fn tycho_chain_to_chain_id(chain: &TychoChain) -> u64 {
    match chain {
        TychoChain::Ethereum => 1,
        TychoChain::Base => 8453,
        TychoChain::Unichain => 130,
        TychoChain::Arbitrum => 42161,
        TychoChain::ZkSync => 324,
        // Default to Ethereum for unknown chains
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_token() -> TychoToken {
        let address = Bytes::from_str("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        TychoToken::new(
            &address,
            "USDC",
            6,
            0,
            &[Some(50000u64)],
            TychoChain::Ethereum,
            100,
        )
    }

    #[test]
    fn test_token_info_from_tycho() {
        let token = create_test_token();
        let info = TokenInfo::from_tycho(&token);

        assert_eq!(
            info.address.to_lowercase(),
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        );
        assert_eq!(info.decimals, 6);
        assert_eq!(info.symbol, "USDC");
        assert_eq!(info.quality, 100);
    }

    #[test]
    fn test_token_info_to_tycho() {
        let info = TokenInfo {
            address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
            decimals: 6,
            symbol: "USDC".to_string(),
            quality: 100,
            chain: Chain::Ethereum,
        };
        let result = info.to_tycho();
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.symbol, "USDC");
        assert_eq!(token.decimals, 6);
        assert_eq!(token.quality, 100);
    }

    #[test]
    fn test_token_info_invalid_address() {
        let info = TokenInfo {
            address: "not_a_valid_address".to_string(),
            decimals: 18,
            symbol: "BAD".to_string(),
            quality: 0,
            chain: Chain::Ethereum,
        };
        let result = info.to_tycho();
        assert!(result.is_err());
        match result {
            Err(ConversionError::InvalidAddress(addr)) => {
                assert!(addr.contains("not_a_valid_address"));
            }
            _ => panic!("Expected InvalidAddress error"),
        }
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = create_test_token();
        let info = TokenInfo::from_tycho(&original);
        let recovered = info.to_tycho_with_chain(TychoChain::Ethereum).unwrap();

        // Address, symbol, decimals, and quality should match
        assert_eq!(original.address, recovered.address);
        assert_eq!(original.symbol, recovered.symbol);
        assert_eq!(original.decimals, recovered.decimals);
        assert_eq!(original.quality, recovered.quality);
    }

    #[test]
    fn test_parse_address_with_prefix() {
        let result = parse_address("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_address_without_prefix() {
        let result = parse_address("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_address_uppercase() {
        let result = parse_address("0xA0B86991C6218B36C1D19D4A2E9EB0CE3606EB48");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_address_invalid() {
        let result = parse_address("0xZZZ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_address_empty() {
        let result = parse_address("");
        assert!(result.is_err());
        let result = parse_address("0x");
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_address() {
        assert_eq!(normalize_address("0xAbCd"), "abcd");
        assert_eq!(normalize_address("0XABCD"), "abcd");
        assert_eq!(normalize_address("abcd"), "abcd");
    }

    #[test]
    fn test_format_address() {
        let address = Bytes::from_str("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let formatted = format_address(&address);
        assert!(formatted.starts_with("0x"));
        assert_eq!(formatted.len(), 42); // 0x + 40 hex chars
    }

    #[test]
    fn test_is_valid_address() {
        assert!(is_valid_address(
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        ));
        assert!(is_valid_address("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"));
        assert!(!is_valid_address("not_valid"));
        assert!(!is_valid_address("0xZZZ"));
    }

    #[test]
    fn test_chain_id_to_tycho_chain() {
        assert!(matches!(
            chain_id_to_tycho_chain(1),
            Ok(TychoChain::Ethereum)
        ));
        assert!(matches!(
            chain_id_to_tycho_chain(8453),
            Ok(TychoChain::Base)
        ));
        assert!(matches!(
            chain_id_to_tycho_chain(130),
            Ok(TychoChain::Unichain)
        ));
        assert!(matches!(
            chain_id_to_tycho_chain(42161),
            Ok(TychoChain::Arbitrum)
        ));
        assert!(chain_id_to_tycho_chain(999).is_err());
    }

    #[test]
    fn test_tycho_chain_to_chain_id() {
        assert_eq!(tycho_chain_to_chain_id(&TychoChain::Ethereum), 1);
        assert_eq!(tycho_chain_to_chain_id(&TychoChain::Base), 8453);
        assert_eq!(tycho_chain_to_chain_id(&TychoChain::Unichain), 130);
        assert_eq!(tycho_chain_to_chain_id(&TychoChain::Arbitrum), 42161);
    }

    #[test]
    fn test_token_info_new() {
        let token = TokenInfo::new("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", "USDC", 6);
        assert_eq!(token.symbol, "USDC");
        assert_eq!(token.decimals, 6);
        assert_eq!(token.quality, 100); // Default quality
    }
}
