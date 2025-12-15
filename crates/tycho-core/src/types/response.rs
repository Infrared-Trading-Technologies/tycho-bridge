//! Native response types for bridge operations.
//!
//! These types are transport-agnostic and returned by BridgeService.
//! Transport layers (gRPC, FFI) convert these to their specific types.

use num_bigint::BigUint;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Serialize BigUint as a decimal string.
fn serialize_biguint_as_string<S>(value: &BigUint, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

/// Deserialize BigUint from a decimal string.
fn deserialize_biguint_from_string<'de, D>(deserializer: D) -> Result<BigUint, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

/// Response for spot price query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotPriceResponse {
    /// Price numerator
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub numerator: BigUint,
    /// Price denominator
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub denominator: BigUint,
}

impl SpotPriceResponse {
    /// Convert to f64 price.
    pub fn to_f64(&self) -> f64 {
        if self.denominator == BigUint::ZERO {
            return 0.0;
        }
        // Convert to f64 for division
        let num = self.numerator.to_string().parse::<f64>().unwrap_or(0.0);
        let den = self.denominator.to_string().parse::<f64>().unwrap_or(1.0);
        num / den
    }
}

/// Response for get_amount_out simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAmountOutResponse {
    /// Output amount
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub amount_out: BigUint,
    /// Estimated gas cost
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub gas_cost: BigUint,
    /// State ID if state was saved
    pub state_id: Option<Uuid>,
    /// Execution price (amount_out / amount_in)
    pub execution_price: Option<ExecutionPrice>,
}

/// Execution price as a rational number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPrice {
    /// Price numerator
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub numerator: BigUint,
    /// Price denominator
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub denominator: BigUint,
}

/// Result for a single simulation (success or error).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationResult {
    /// Successful simulation
    Success(GetAmountOutResponse),
    /// Failed simulation
    Error(SimulationErrorInfo),
}

/// Information about a simulation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationErrorInfo {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
}

/// Response for batch get_amount_out simulations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGetAmountOutResponse {
    /// Results for each request (in same order, None if failed)
    pub results: Vec<Option<GetAmountOutResponse>>,
}

/// Response for fee query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFeeResponse {
    /// Fee as a fraction (numerator/denominator)
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub fee_numerator: BigUint,
    /// Fee denominator
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub fee_denominator: BigUint,
}

impl GetFeeResponse {
    /// Convert to f64 fee.
    pub fn to_f64(&self) -> f64 {
        if self.fee_denominator == BigUint::ZERO {
            return 0.0;
        }
        let num = self.fee_numerator.to_string().parse::<f64>().unwrap_or(0.0);
        let den = self
            .fee_denominator
            .to_string()
            .parse::<f64>()
            .unwrap_or(1.0);
        num / den
    }
}

/// Response for limits query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLimitsResponse {
    /// Minimum input amount
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub min_amount_in: BigUint,
    /// Maximum input amount
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub max_amount_in: BigUint,
}

/// Trade information from swap_to_price or query_supply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeInfo {
    /// Input amount
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub amount_in: BigUint,
    /// Output amount
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub amount_out: BigUint,
    /// Price after trade (numerator)
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub price_numerator: BigUint,
    /// Price after trade (denominator)
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub price_denominator: BigUint,
}

/// Response for swap_to_price simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapToPriceResponse {
    /// Trade information
    pub trade: Option<TradeInfo>,
    /// Whether the target price was reached
    pub reached_target: bool,
}

/// Response for query_supply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySupplyResponse {
    /// Supply curve points
    pub trades: Vec<TradeInfo>,
}

/// Result for a single hop in route simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopResult {
    /// Component ID
    pub component_id: String,
    /// Amount out from this hop
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub amount_out: BigUint,
    /// Gas cost for this hop
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub gas_cost: BigUint,
}

/// Response for multi-hop route simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateRouteResponse {
    /// Final amount out
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub final_amount_out: BigUint,
    /// Total gas cost
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub total_gas_cost: BigUint,
    /// Per-hop results
    pub hop_results: Vec<HopResult>,
}

/// Token information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token address
    pub address: String,
    /// Token symbol
    pub symbol: String,
    /// Token decimals
    pub decimals: u32,
    /// Chain ID
    pub chain_id: u64,
}

/// Response for getting tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTokensResponse {
    /// List of tokens
    pub tokens: Vec<TokenInfo>,
}

/// Protocol component information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// Component ID
    pub id: String,
    /// Protocol system name
    pub protocol_system: String,
    /// Protocol type name
    pub protocol_type_name: String,
    /// Chain ID
    pub chain_id: u64,
    /// Token addresses in this component
    pub tokens: Vec<String>,
    /// TVL in USD (if available)
    pub tvl_usd: Option<f64>,
    /// Creation timestamp
    pub created_at: i64,
}

/// Response for getting protocol components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProtocolComponentsResponse {
    /// List of components
    pub components: Vec<ComponentInfo>,
}

/// Chain status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatus {
    /// Chain ID
    pub chain_id: u64,
    /// Latest synced block
    pub latest_block: u64,
    /// Number of components
    pub component_count: usize,
    /// Whether snapshot is complete
    pub snapshot_complete: bool,
    /// Connection status
    pub connected: bool,
}

/// Response for status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusResponse {
    /// Sync status
    pub sync_status: SyncStatus,
    /// Latest block from chain
    pub latest_block: u64,
    /// Currently synced block
    pub synced_block: u64,
    /// Number of components
    pub component_count: usize,
    /// Registered protocol systems
    pub protocol_systems: Vec<String>,
}

/// Response for solution encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeSolutionResponse {
    /// Encoded calldata
    pub calldata: Vec<u8>,
    /// Router address
    pub router_address: String,
    /// Value to send (for ETH transactions)
    pub value: BigUint,
}

/// A streaming update event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUpdate {
    /// Update type
    pub update_type: UpdateType,
    /// Chain ID
    pub chain_id: u64,
    /// Component ID
    pub component_id: String,
    /// Block number
    pub block_number: u64,
    /// Component data (if New or Modified)
    pub component: Option<ComponentInfo>,
}

/// Type of streaming update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateType {
    /// New component added
    New,
    /// Existing component modified
    Modified,
    /// Component removed
    Removed,
}

// ============================================================================
// Additional response types for handler compatibility
// ============================================================================

/// Alias for getting components response.
pub type GetComponentsResponse = GetProtocolComponentsResponse;

/// Response for getting a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetComponentResponse {
    /// The component (if found)
    pub component: Option<ComponentInfo>,
}

/// Response for calldata encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeCalldataResponse {
    /// Encoded calldata (hex)
    pub calldata: Vec<u8>,
    /// Target contract address
    pub to: String,
    /// Value to send (for ETH transactions)
    #[serde(
        serialize_with = "serialize_biguint_as_string",
        deserialize_with = "deserialize_biguint_from_string"
    )]
    pub value: BigUint,
}

/// Response for batch calldata encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEncodeCalldataResponse {
    /// Results for each request
    pub results: Vec<Option<EncodeCalldataResponse>>,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Whether the service is healthy
    pub healthy: bool,
    /// Service version
    pub version: String,
    /// Supported chain IDs
    pub chains: Vec<u64>,
}

/// Sync status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Syncing in progress
    Syncing,
    /// Fully synced
    Synced,
    /// Not started
    NotStarted,
    /// Error state
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spot_price_to_f64() {
        let price = SpotPriceResponse {
            numerator: BigUint::from(3u64),
            denominator: BigUint::from(2u64),
        };
        assert!((price.to_f64() - 1.5).abs() < 0.0001);
    }

    #[test]
    fn test_spot_price_to_f64_zero_denominator() {
        let price = SpotPriceResponse {
            numerator: BigUint::from(100u64),
            denominator: BigUint::ZERO,
        };
        assert_eq!(price.to_f64(), 0.0);
    }

    #[test]
    fn test_get_fee_to_f64() {
        let fee = GetFeeResponse {
            fee_numerator: BigUint::from(3u64),
            fee_denominator: BigUint::from(1000u64),
        };
        assert!((fee.to_f64() - 0.003).abs() < 0.0001);
    }

    #[test]
    fn test_get_fee_to_f64_zero_denominator() {
        let fee = GetFeeResponse {
            fee_numerator: BigUint::from(3u64),
            fee_denominator: BigUint::ZERO,
        };
        assert_eq!(fee.to_f64(), 0.0);
    }

    #[test]
    fn test_spot_price_serialization() {
        let price = SpotPriceResponse {
            numerator: BigUint::from(12345u64),
            denominator: BigUint::from(100u64),
        };
        let json = serde_json::to_string(&price).unwrap();
        assert!(json.contains("\"12345\""));
        assert!(json.contains("\"100\""));

        let deserialized: SpotPriceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.numerator, BigUint::from(12345u64));
        assert_eq!(deserialized.denominator, BigUint::from(100u64));
    }

    #[test]
    fn test_simulation_result_success() {
        let resp = GetAmountOutResponse {
            amount_out: BigUint::from(1000u64),
            gas_cost: BigUint::from(50000u64),
            state_id: None,
            execution_price: None,
        };
        let result = SimulationResult::Success(resp);

        if let SimulationResult::Success(r) = result {
            assert_eq!(r.amount_out, BigUint::from(1000u64));
        } else {
            panic!("Expected Success variant");
        }
    }

    #[test]
    fn test_simulation_result_error() {
        let result = SimulationResult::Error(SimulationErrorInfo {
            code: 4,
            message: "Simulation failed".to_string(),
        });

        if let SimulationResult::Error(e) = result {
            assert_eq!(e.code, 4);
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_sync_status_equality() {
        assert_eq!(SyncStatus::Syncing, SyncStatus::Syncing);
        assert_eq!(SyncStatus::Synced, SyncStatus::Synced);
        assert_ne!(SyncStatus::Syncing, SyncStatus::Synced);
    }

    #[test]
    fn test_update_type_equality() {
        assert_eq!(UpdateType::New, UpdateType::New);
        assert_eq!(UpdateType::Modified, UpdateType::Modified);
        assert_ne!(UpdateType::New, UpdateType::Removed);
    }

    #[test]
    fn test_health_check_response() {
        let resp = HealthCheckResponse {
            healthy: true,
            version: "1.0.0".to_string(),
            chains: vec![1, 8453],
        };
        assert!(resp.healthy);
        assert_eq!(resp.chains.len(), 2);
    }

    #[test]
    fn test_batch_response_with_mixed_results() {
        let batch = BatchGetAmountOutResponse {
            results: vec![
                Some(GetAmountOutResponse {
                    amount_out: BigUint::from(100u64),
                    gas_cost: BigUint::from(1000u64),
                    state_id: None,
                    execution_price: None,
                }),
                None, // Failed simulation
                Some(GetAmountOutResponse {
                    amount_out: BigUint::from(200u64),
                    gas_cost: BigUint::from(2000u64),
                    state_id: None,
                    execution_price: None,
                }),
            ],
        };
        assert_eq!(batch.results.len(), 3);
        assert!(batch.results[0].is_some());
        assert!(batch.results[1].is_none());
        assert!(batch.results[2].is_some());
    }
}
