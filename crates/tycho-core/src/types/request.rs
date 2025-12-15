//! Native request types for bridge operations.
//!
//! These types are transport-agnostic and used by BridgeService.
//! Transport layers (gRPC, FFI) convert their specific types to/from these.

use num_bigint::BigUint;
use uuid::Uuid;

/// Request for spot price query.
#[derive(Debug, Clone)]
pub struct SpotPriceRequest {
    /// Chain ID (e.g., 1 for Ethereum, 8453 for Base)
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Base token address (token to price)
    pub base_token: String,
    /// Quote token address (price denomination)
    pub quote_token: String,
}

/// Request for get_amount_out simulation.
#[derive(Debug, Clone)]
pub struct GetAmountOutRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
    /// Amount of input token
    pub amount_in: BigUint,
    /// Optional state ID for state chaining
    pub state_id: Option<Uuid>,
    /// Whether to save the resulting state
    pub save_state: bool,
}

/// Request for batch get_amount_out simulations.
#[derive(Debug, Clone)]
pub struct BatchGetAmountOutRequest {
    /// List of individual simulation requests
    pub requests: Vec<GetAmountOutRequest>,
}

/// Request for fee query.
#[derive(Debug, Clone)]
pub struct GetFeeRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Token address to get fee for
    pub token: String,
}

/// Request for limits query.
#[derive(Debug, Clone)]
pub struct GetLimitsRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
}

/// Request for swap_to_price simulation.
#[derive(Debug, Clone)]
pub struct SwapToPriceRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
    /// Target price numerator
    pub target_price_numerator: BigUint,
    /// Target price denominator
    pub target_price_denominator: BigUint,
}

/// Request for query_supply.
#[derive(Debug, Clone)]
pub struct QuerySupplyRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
    /// Minimum price numerator (floor price)
    pub min_price_numerator: BigUint,
    /// Minimum price denominator
    pub min_price_denominator: BigUint,
}

/// A single hop in a route simulation.
#[derive(Debug, Clone)]
pub struct RouteHop {
    /// Component/pool ID for this hop
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
}

/// Request for multi-hop route simulation.
#[derive(Debug, Clone)]
pub struct SimulateRouteRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Ordered list of hops to simulate
    pub hops: Vec<RouteHop>,
    /// Initial amount in
    pub amount_in: BigUint,
    /// Pre-existing state overrides (component_id -> state UUID)
    pub state_overrides: std::collections::HashMap<String, uuid::Uuid>,
}

/// Request for getting tokens.
#[derive(Debug, Clone)]
pub struct GetTokensRequest {
    /// Chain ID (optional, if not provided returns all chains)
    pub chain_id: Option<u64>,
    /// Token addresses to filter (optional)
    pub addresses: Vec<String>,
    /// Maximum number of tokens to return
    pub limit: Option<u32>,
}

/// Request for getting protocol components.
#[derive(Debug, Clone)]
pub struct GetProtocolComponentsRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component IDs to filter (optional)
    pub component_ids: Vec<String>,
    /// Minimum TVL filter
    pub min_tvl: Option<f64>,
    /// Protocol type filter (e.g., "uniswap_v2", "uniswap_v3")
    pub protocol_type: Option<String>,
    /// Maximum number of components to return
    pub limit: Option<u32>,
}

/// Request for status.
#[derive(Debug, Clone, Default)]
pub struct GetStatusRequest {
    /// Chain ID to query status for
    pub chain_id: u64,
    /// Whether to include detailed chain status
    pub include_chain_details: bool,
}

/// Swap information for encoding.
#[derive(Debug, Clone)]
pub struct SwapInfo {
    /// Component/pool ID
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
    /// Amount in
    pub amount_in: BigUint,
    /// Minimum amount out (slippage protection)
    pub min_amount_out: BigUint,
    /// Component state (optional override)
    pub component: Option<ComponentInfo>,
}

/// Component information for encoding.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// Component ID
    pub id: String,
    /// Protocol system name
    pub protocol_system: String,
    /// Token addresses
    pub tokens: Vec<String>,
    /// Static attributes (key-value pairs serialized)
    pub static_attributes: Vec<(String, Vec<u8>)>,
}

/// User transfer type for solution encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserTransferType {
    /// Transfer tokens from user
    TransferFrom,
    /// Use Permit2 for transfer
    Permit2,
    /// No transfer needed (tokens already in contract)
    None,
}

/// Native action for wrap/unwrap ETH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAction {
    /// No native action
    None,
    /// Wrap ETH to WETH
    Wrap,
    /// Unwrap WETH to ETH
    Unwrap,
}

/// Encoder type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderType {
    /// Tycho router encoder
    TychoRouter,
}

/// Permit data for Permit2 transfers.
#[derive(Debug, Clone)]
pub struct Permit {
    /// Token address
    pub token: String,
    /// Permitted amount
    pub amount: BigUint,
    /// Permit expiration timestamp
    pub expiration: u64,
    /// Permit nonce
    pub nonce: u64,
    /// Permit signature
    pub signature: Vec<u8>,
}

/// Request for solution encoding.
#[derive(Debug, Clone)]
pub struct EncodeSolutionRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Sender address
    pub sender: String,
    /// Receiver address
    pub receiver: String,
    /// Input token address
    pub given_token: String,
    /// Given amount
    pub given_amount: BigUint,
    /// Whether given is input (true) or output (false)
    pub given_is_input: bool,
    /// Exact output amount (if given_is_input is false)
    pub exact_out_amount: Option<BigUint>,
    /// Checked token address
    pub checked_token: String,
    /// Expected amount for checked token
    pub expected_amount: BigUint,
    /// Is expected amount minimum (true) or maximum (false)
    pub is_expected_min: bool,
    /// Swaps to encode
    pub swaps: Vec<SwapInfo>,
    /// User transfer type
    pub user_transfer_type: UserTransferType,
    /// Native action
    pub native_action: NativeAction,
    /// Encoder type
    pub encoder_type: EncoderType,
    /// Permit data (for Permit2)
    pub permit: Option<Permit>,
    /// Router address override
    pub router_address: Option<String>,
}

/// Request for streaming updates.
#[derive(Debug, Clone)]
pub struct StreamUpdatesRequest {
    /// Chain IDs to subscribe to (empty = all)
    pub chain_ids: Vec<u64>,
    /// Component IDs to subscribe to (empty = all)
    pub component_ids: Vec<String>,
    /// Whether to include initial snapshot
    pub include_snapshot: bool,
}

/// Request for getting protocol components (alias for consistency).
pub type GetComponentsRequest = GetProtocolComponentsRequest;

/// Request for getting a single component.
#[derive(Debug, Clone)]
pub struct GetComponentRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component ID
    pub component_id: String,
}

/// Request for encoding calldata.
#[derive(Debug, Clone)]
pub struct EncodeCalldataRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: String,
    /// Input token address
    pub token_in: String,
    /// Output token address
    pub token_out: String,
    /// Amount in
    pub amount_in: BigUint,
    /// Minimum amount out (slippage protection)
    pub min_amount_out: BigUint,
    /// Recipient address
    pub recipient: String,
}

/// Request for batch calldata encoding.
#[derive(Debug, Clone)]
pub struct BatchEncodeCalldataRequest {
    /// Individual encoding requests
    pub requests: Vec<EncodeCalldataRequest>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn test_spot_price_request_construction() {
        let req = SpotPriceRequest {
            chain_id: 1,
            component_id: "pool_123".to_string(),
            base_token: "0xtoken_a".to_string(),
            quote_token: "0xtoken_b".to_string(),
        };
        assert_eq!(req.chain_id, 1);
        assert_eq!(req.component_id, "pool_123");
    }

    #[test]
    fn test_get_amount_out_request_with_state_id() {
        let state_id = Uuid::new_v4();
        let req = GetAmountOutRequest {
            chain_id: 8453,
            component_id: "pool".to_string(),
            token_in: "0xa".to_string(),
            token_out: "0xb".to_string(),
            amount_in: BigUint::from(1000u64),
            state_id: Some(state_id),
            save_state: true,
        };
        assert_eq!(req.state_id, Some(state_id));
        assert!(req.save_state);
    }

    #[test]
    fn test_get_status_request_default() {
        let req = GetStatusRequest::default();
        assert_eq!(req.chain_id, 0);
        assert!(!req.include_chain_details);
    }

    #[test]
    fn test_user_transfer_type_equality() {
        assert_eq!(UserTransferType::TransferFrom, UserTransferType::TransferFrom);
        assert_ne!(UserTransferType::TransferFrom, UserTransferType::Permit2);
        assert_ne!(UserTransferType::Permit2, UserTransferType::None);
    }

    #[test]
    fn test_native_action_equality() {
        assert_eq!(NativeAction::None, NativeAction::None);
        assert_eq!(NativeAction::Wrap, NativeAction::Wrap);
        assert_ne!(NativeAction::Wrap, NativeAction::Unwrap);
    }

    #[test]
    fn test_encoder_type() {
        let encoder = EncoderType::TychoRouter;
        assert_eq!(encoder, EncoderType::TychoRouter);
    }

    #[test]
    fn test_route_hop_construction() {
        let hop = RouteHop {
            component_id: "pool_1".to_string(),
            token_in: "0xa".to_string(),
            token_out: "0xb".to_string(),
        };
        let cloned = hop.clone();
        assert_eq!(hop.component_id, cloned.component_id);
    }

    #[test]
    fn test_batch_request_construction() {
        let batch = BatchGetAmountOutRequest {
            requests: vec![
                GetAmountOutRequest {
                    chain_id: 1,
                    component_id: "p1".to_string(),
                    token_in: "a".to_string(),
                    token_out: "b".to_string(),
                    amount_in: BigUint::from(100u64),
                    state_id: None,
                    save_state: false,
                },
                GetAmountOutRequest {
                    chain_id: 1,
                    component_id: "p2".to_string(),
                    token_in: "b".to_string(),
                    token_out: "c".to_string(),
                    amount_in: BigUint::from(200u64),
                    state_id: None,
                    save_state: false,
                },
            ],
        };
        assert_eq!(batch.requests.len(), 2);
    }

    #[test]
    fn test_simulate_route_request_with_overrides() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("pool_1".to_string(), Uuid::new_v4());

        let req = SimulateRouteRequest {
            chain_id: 1,
            hops: vec![],
            amount_in: BigUint::from(1000u64),
            state_overrides: overrides,
        };
        assert_eq!(req.state_overrides.len(), 1);
    }
}
