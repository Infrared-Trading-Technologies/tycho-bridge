//! ProtocolComponent conversion utilities.
//!
//! Provides transport-agnostic types and utilities for working with protocol components.
//!
//! # Tycho SDK ProtocolComponent
//!
//! From tycho-simulation:
//! ```ignore
//! pub struct ProtocolComponent {
//!     pub id: Bytes,
//!     pub tokens: Vec<Token>,
//!     pub protocol_system: String,
//!     pub protocol_type_name: String,
//!     pub chain: Chain,
//!     pub contract_ids: Vec<Bytes>,
//!     pub static_attributes: HashMap<String, Bytes>,
//!     pub creation_tx: Bytes,
//!     pub created_at: NaiveDateTime,
//! }
//! ```
//!
//! Source: https://docs.propellerheads.xyz/tycho/concepts (ProtocolComponents section)

use std::collections::HashMap;
use std::str::FromStr;

use tycho_simulation::tycho_common::Bytes;

use super::token::tycho_chain_to_chain_id;
use super::{ConversionError, ConversionResult};

// Import the Tycho SDK types
use tycho_simulation::protocol::models::ProtocolComponent as TychoComponent;

// Import EvmChain for EVM-specific conversions
use tycho_simulation::evm::tycho_models::Chain as EvmChain;

// Re-export Tycho's Chain type for external use
pub use tycho_simulation::tycho_common::models::Chain as TychoChain;

/// Native Chain enum - transport-agnostic representation.
/// This can be serialized to JSON, FFI, or converted to proto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    /// Unspecified chain (default)
    Unspecified,
    /// Ethereum mainnet (chain_id: 1)
    Ethereum,
    /// Base L2 (chain_id: 8453)
    Base,
    /// Unichain L2 (chain_id: 130)
    Unichain,
    /// Arbitrum One (chain_id: 42161)
    Arbitrum,
}

impl Default for Chain {
    fn default() -> Self {
        Chain::Unspecified
    }
}

impl Chain {
    /// Get the chain ID for this chain.
    pub fn chain_id(&self) -> u64 {
        match self {
            Chain::Ethereum => 1,
            Chain::Base => 8453,
            Chain::Unichain => 130,
            Chain::Arbitrum => 42161,
            Chain::Unspecified => 0,
        }
    }

    /// Create a Chain from a chain ID.
    pub fn from_chain_id(chain_id: u64) -> Self {
        match chain_id {
            1 => Chain::Ethereum,
            8453 => Chain::Base,
            130 => Chain::Unichain,
            42161 => Chain::Arbitrum,
            _ => Chain::Unspecified,
        }
    }

    /// Convert from Tycho SDK Chain.
    pub fn from_tycho(chain: &TychoChain) -> Self {
        match chain {
            TychoChain::Ethereum => Chain::Ethereum,
            TychoChain::Base => Chain::Base,
            TychoChain::Unichain => Chain::Unichain,
            TychoChain::Arbitrum => Chain::Arbitrum,
            _ => Chain::Unspecified,
        }
    }

    /// Convert to Tycho SDK Chain.
    pub fn to_tycho(&self) -> ConversionResult<TychoChain> {
        match self {
            Chain::Ethereum => Ok(TychoChain::Ethereum),
            Chain::Base => Ok(TychoChain::Base),
            Chain::Unichain => Ok(TychoChain::Unichain),
            Chain::Arbitrum => Ok(TychoChain::Arbitrum),
            Chain::Unspecified => Err(ConversionError::UnknownChain(0)),
        }
    }
}

impl From<TychoChain> for Chain {
    fn from(chain: TychoChain) -> Self {
        Chain::from_tycho(&chain)
    }
}

/// Convert an EvmChain to a native Chain.
pub fn evm_chain_to_native(chain: &EvmChain) -> Chain {
    match chain {
        EvmChain::Ethereum => Chain::Ethereum,
        EvmChain::Base => Chain::Base,
        EvmChain::Unichain => Chain::Unichain,
        EvmChain::Arbitrum => Chain::Arbitrum,
        _ => Chain::Unspecified,
    }
}

/// Convert an EvmChain to a chain ID.
pub fn evm_chain_to_u64(chain: &EvmChain) -> u64 {
    match chain {
        EvmChain::Ethereum => 1,
        EvmChain::Base => 8453,
        EvmChain::Unichain => 130,
        EvmChain::Arbitrum => 42161,
        _ => 0,
    }
}

/// Parse a hex string to Bytes.
///
/// Handles both `0x` prefixed and non-prefixed strings.
pub fn parse_bytes(hex_str: &str) -> ConversionResult<Bytes> {
    let normalized = hex_str
        .strip_prefix("0x")
        .or_else(|| hex_str.strip_prefix("0X"))
        .unwrap_or(hex_str)
        .to_lowercase();

    if normalized.is_empty() {
        return Ok(Bytes::default());
    }

    Bytes::from_str(&normalized).map_err(|_| ConversionError::InvalidAddress(hex_str.to_string()))
}

/// Format Bytes as a hex string with 0x prefix.
pub fn format_bytes(bytes: &Bytes) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Extract protocol info from a component for quick lookups.
///
/// # Returns
///
/// A tuple of (id, protocol_system, chain_id)
pub fn component_key_info(component: &TychoComponent) -> (String, String, u64) {
    let id = format!("0x{}", hex::encode(&component.id));
    let protocol_system = component.protocol_system.clone();
    let chain_id = tycho_chain_to_chain_id(&component.chain);
    (id, protocol_system, chain_id)
}

/// Check if a component matches a protocol filter.
pub fn component_matches_protocol(component: &TychoComponent, protocols: &[String]) -> bool {
    if protocols.is_empty() {
        return true;
    }
    protocols.contains(&component.protocol_system)
}

/// Check if a component contains any of the specified token addresses.
pub fn component_contains_tokens(component: &TychoComponent, token_addresses: &[String]) -> bool {
    if token_addresses.is_empty() {
        return true;
    }

    let component_tokens: Vec<String> = component
        .tokens
        .iter()
        .map(|t| format!("0x{}", hex::encode(&t.address)).to_lowercase())
        .collect();

    token_addresses
        .iter()
        .any(|addr| component_tokens.contains(&addr.to_lowercase()))
}

/// Native ProtocolComponent representation - transport agnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProtocolComponentInfo {
    /// Component ID (hex string with 0x prefix)
    pub id: String,
    /// Protocol system name (e.g., "uniswap_v2")
    pub protocol_system: String,
    /// Protocol type name (e.g., "UniswapV2Pool")
    pub protocol_type_name: String,
    /// Chain
    pub chain: Chain,
    /// Token addresses (hex strings with 0x prefix)
    pub tokens: Vec<String>,
    /// Contract addresses (hex strings with 0x prefix)
    pub contract_addresses: Vec<String>,
    /// Static attributes as hex strings
    pub static_attributes: HashMap<String, String>,
    /// Creation transaction hash
    pub creation_tx: String,
}

impl ProtocolComponentInfo {
    /// Create from a Tycho SDK component.
    pub fn from_tycho(component: &TychoComponent) -> Self {
        Self {
            id: format!("0x{}", hex::encode(&component.id)),
            protocol_system: component.protocol_system.clone(),
            protocol_type_name: component.protocol_type_name.clone(),
            chain: Chain::from_tycho(&component.chain),
            tokens: component
                .tokens
                .iter()
                .map(|t| format!("0x{}", hex::encode(&t.address)))
                .collect(),
            contract_addresses: component
                .contract_ids
                .iter()
                .map(|c| format!("0x{}", hex::encode(c)))
                .collect(),
            static_attributes: component
                .static_attributes
                .iter()
                .map(|(k, v)| (k.clone(), format!("0x{}", hex::encode(v))))
                .collect(),
            creation_tx: format!("0x{}", hex::encode(&component.creation_tx)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use tycho_simulation::tycho_common::models::token::Token as TychoToken;

    fn create_test_component() -> TychoComponent {
        let id = Bytes::from_str("b4e16d0168e52d35cacd2c6185b44281ec28c9dc").unwrap();
        let token_addr1 = Bytes::from_str("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let token_addr2 = Bytes::from_str("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap();

        let tokens = vec![
            TychoToken::new(&token_addr1, "USDC", 6, 0, &[], TychoChain::Ethereum, 100),
            TychoToken::new(&token_addr2, "WETH", 18, 0, &[], TychoChain::Ethereum, 100),
        ];

        let mut static_attrs = HashMap::new();
        static_attrs.insert(
            "fee".to_string(),
            Bytes::from_str("0bb8").unwrap(), // 3000 = 0.3%
        );

        #[allow(deprecated)]
        TychoComponent {
            address: id.clone(),
            id,
            tokens,
            protocol_system: "uniswap_v2".to_string(),
            protocol_type_name: "UniswapV2Pool".to_string(),
            chain: TychoChain::Ethereum,
            contract_ids: vec![],
            static_attributes: static_attrs,
            creation_tx: Bytes::default(),
            created_at: NaiveDateTime::default(),
        }
    }

    #[test]
    fn test_chain_conversions() {
        // Chain ID conversions
        assert_eq!(Chain::from_chain_id(1), Chain::Ethereum);
        assert_eq!(Chain::from_chain_id(8453), Chain::Base);
        assert_eq!(Chain::from_chain_id(130), Chain::Unichain);
        assert_eq!(Chain::from_chain_id(42161), Chain::Arbitrum);
        assert_eq!(Chain::from_chain_id(999), Chain::Unspecified);

        // Chain ID round trip
        assert_eq!(Chain::Ethereum.chain_id(), 1);
        assert_eq!(Chain::Base.chain_id(), 8453);
        assert_eq!(Chain::Unichain.chain_id(), 130);
        assert_eq!(Chain::Arbitrum.chain_id(), 42161);
    }

    #[test]
    fn test_tycho_chain_conversions() {
        assert_eq!(Chain::from_tycho(&TychoChain::Ethereum), Chain::Ethereum);
        assert_eq!(Chain::from_tycho(&TychoChain::Base), Chain::Base);

        assert!(Chain::Ethereum.to_tycho().is_ok());
        assert!(Chain::Unspecified.to_tycho().is_err());
    }

    #[test]
    fn test_parse_bytes_with_prefix() {
        let result = parse_bytes("0xabcdef");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_bytes_without_prefix() {
        let result = parse_bytes("abcdef");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_bytes_empty() {
        let result = parse_bytes("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Bytes::default());
    }

    #[test]
    fn test_parse_bytes_invalid() {
        let result = parse_bytes("0xZZZZ");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_bytes() {
        let bytes = Bytes::from_str("abcdef").unwrap();
        let formatted = format_bytes(&bytes);
        assert!(formatted.starts_with("0x"));
        assert!(formatted.contains("abcdef"));
    }

    #[test]
    fn test_component_key_info() {
        let component = create_test_component();
        let (id, protocol, chain_id) = component_key_info(&component);

        assert!(id
            .to_lowercase()
            .contains("b4e16d0168e52d35cacd2c6185b44281ec28c9dc"));
        assert_eq!(protocol, "uniswap_v2");
        assert_eq!(chain_id, 1); // Ethereum
    }

    #[test]
    fn test_component_matches_protocol() {
        let component = create_test_component();

        assert!(component_matches_protocol(&component, &[])); // Empty = match all
        assert!(component_matches_protocol(
            &component,
            &["uniswap_v2".to_string()]
        ));
        assert!(!component_matches_protocol(
            &component,
            &["uniswap_v3".to_string()]
        ));
        assert!(component_matches_protocol(
            &component,
            &["sushiswap".to_string(), "uniswap_v2".to_string()]
        ));
    }

    #[test]
    fn test_component_contains_tokens() {
        let component = create_test_component();

        assert!(component_contains_tokens(&component, &[])); // Empty = match all
        assert!(component_contains_tokens(
            &component,
            &["0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string()] // USDC
        ));
        assert!(component_contains_tokens(
            &component,
            &["0xC02AAA39B223FE8D0A0E5C4F27EAD9083C756CC2".to_string()] // WETH (uppercase)
        ));
        assert!(!component_contains_tokens(
            &component,
            &["0x0000000000000000000000000000000000000000".to_string()] // Not in component
        ));
    }

    #[test]
    fn test_protocol_component_info() {
        let component = create_test_component();
        let info = ProtocolComponentInfo::from_tycho(&component);

        assert!(info
            .id
            .to_lowercase()
            .contains("b4e16d0168e52d35cacd2c6185b44281ec28c9dc"));
        assert_eq!(info.protocol_system, "uniswap_v2");
        assert_eq!(info.chain, Chain::Ethereum);
        assert_eq!(info.tokens.len(), 2);
    }
}
