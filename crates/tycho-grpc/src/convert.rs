//! Proto ↔ Native type conversions.
//!
//! This module handles conversions between proto types and tycho-core native types.
//! Implements `From` traits for seamless conversion between gRPC and core layers.

use num_bigint::BigUint;
use num_traits::Zero;
use std::str::FromStr;
use uuid::Uuid;

use tycho_core::error::BridgeError;
use tycho_core::types::request as req;
use tycho_core::types::response as resp;

use crate::proto::{self, BigInt, Chain, Price};

// ─────────────────────────────────────────────────────────────────────────────
// Chain Conversions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert proto Chain enum to u64 chain ID.
pub fn chain_to_u64(chain: Chain) -> u64 {
    match chain {
        Chain::Unspecified => 0,
        Chain::Ethereum => 1,
        Chain::Base => 8453,
        Chain::Unichain => 130,
        Chain::Arbitrum => 42161,
    }
}

/// Convert u64 chain ID to proto Chain enum.
pub fn u64_to_chain(chain_id: u64) -> Chain {
    match chain_id {
        1 => Chain::Ethereum,
        8453 => Chain::Base,
        130 => Chain::Unichain,
        42161 => Chain::Arbitrum,
        _ => Chain::Unspecified,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BigInt Conversions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert BigUint to proto BigInt.
pub fn biguint_to_proto(value: &BigUint) -> BigInt {
    BigInt {
        value: value.to_string(),
    }
}

/// Parse proto BigInt to BigUint.
pub fn proto_to_biguint(proto: &Option<BigInt>) -> Result<BigUint, BridgeError> {
    let bi = proto
        .as_ref()
        .ok_or_else(|| BridgeError::invalid_argument("Missing BigInt"))?;

    if bi.value.is_empty() {
        return Ok(BigUint::zero());
    }

    BigUint::from_str(&bi.value)
        .map_err(|e| BridgeError::invalid_argument(format!("Invalid BigInt: {}", e)))
}

/// Parse proto BigInt to BigUint, returning zero if missing.
pub fn proto_to_biguint_or_zero(proto: &Option<BigInt>) -> BigUint {
    proto
        .as_ref()
        .and_then(|bi| BigUint::from_str(&bi.value).ok())
        .unwrap_or_else(BigUint::zero)
}

// ─────────────────────────────────────────────────────────────────────────────
// Price Conversions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert numerator/denominator to proto Price.
pub fn to_proto_price(numerator: &BigUint, denominator: &BigUint) -> Price {
    Price {
        numerator: Some(biguint_to_proto(numerator)),
        denominator: Some(biguint_to_proto(denominator)),
    }
}

/// Convert f64 to proto Price (approximate using fixed precision).
pub fn f64_to_proto_price(price: f64) -> Price {
    let price_info = tycho_core::types::from_f64(price);
    let numerator = price_info.numerator.to_biguint().unwrap_or_default();
    let denominator = price_info
        .denominator
        .to_biguint()
        .unwrap_or(BigUint::from(1u32));
    to_proto_price(&numerator, &denominator)
}

// ─────────────────────────────────────────────────────────────────────────────
// UUID Conversions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert UUID to bytes for proto transport.
pub fn uuid_to_bytes(uuid: &Uuid) -> Vec<u8> {
    uuid.as_bytes().to_vec()
}

/// Parse bytes to UUID.
pub fn bytes_to_uuid(bytes: &[u8]) -> Option<Uuid> {
    if bytes.len() != 16 {
        return None;
    }
    let arr: [u8; 16] = bytes.try_into().ok()?;
    Some(Uuid::from_bytes(arr))
}

// ─────────────────────────────────────────────────────────────────────────────
// Request Conversions: Proto → Core
// ─────────────────────────────────────────────────────────────────────────────

impl TryFrom<proto::SpotPriceRequest> for req::SpotPriceRequest {
    type Error = BridgeError;

    fn try_from(p: proto::SpotPriceRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: chain_to_u64(p.chain()),
            component_id: p.component_id,
            base_token: p.base_token,
            quote_token: p.quote_token,
        })
    }
}

impl TryFrom<proto::GetAmountOutRequest> for req::GetAmountOutRequest {
    type Error = BridgeError;

    fn try_from(p: proto::GetAmountOutRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: chain_to_u64(p.chain()),
            component_id: p.component_id,
            token_in: p.token_in,
            token_out: p.token_out,
            amount_in: proto_to_biguint(&p.amount_in)?,
            state_id: p
                .pool_state_override
                .as_ref()
                .and_then(|b| bytes_to_uuid(b)),
            save_state: true, // Always save state for chaining
        })
    }
}

impl TryFrom<proto::GetFeeRequest> for req::GetFeeRequest {
    type Error = BridgeError;

    fn try_from(p: proto::GetFeeRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: chain_to_u64(p.chain()),
            component_id: p.component_id,
            token: String::new(), // GetFeeRequest in proto doesn't have token field
        })
    }
}

impl TryFrom<proto::GetLimitsRequest> for req::GetLimitsRequest {
    type Error = BridgeError;

    fn try_from(p: proto::GetLimitsRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: chain_to_u64(p.chain()),
            component_id: p.component_id,
            token_in: p.sell_token,
            token_out: p.buy_token,
        })
    }
}

impl TryFrom<proto::SwapToPriceRequest> for req::SwapToPriceRequest {
    type Error = BridgeError;

    fn try_from(p: proto::SwapToPriceRequest) -> Result<Self, Self::Error> {
        let chain_id = chain_to_u64(p.chain());
        let target = p
            .target_price
            .ok_or_else(|| BridgeError::invalid_argument("Missing target_price"))?;

        Ok(Self {
            chain_id,
            component_id: p.component_id,
            token_in: p.token_in,
            token_out: p.token_out,
            target_price_numerator: proto_to_biguint(&target.numerator)?,
            target_price_denominator: proto_to_biguint(&target.denominator)?,
        })
    }
}

impl TryFrom<proto::QuerySupplyRequest> for req::QuerySupplyRequest {
    type Error = BridgeError;

    fn try_from(p: proto::QuerySupplyRequest) -> Result<Self, Self::Error> {
        let chain_id = chain_to_u64(p.chain());
        let min_price = p
            .min_price
            .ok_or_else(|| BridgeError::invalid_argument("Missing min_price"))?;

        Ok(Self {
            chain_id,
            component_id: p.component_id,
            token_in: p.token_in,
            token_out: p.token_out,
            min_price_numerator: proto_to_biguint(&min_price.numerator)?,
            min_price_denominator: proto_to_biguint(&min_price.denominator)?,
        })
    }
}

impl TryFrom<proto::SimulateRouteRequest> for req::SimulateRouteRequest {
    type Error = BridgeError;

    fn try_from(p: proto::SimulateRouteRequest) -> Result<Self, Self::Error> {
        let chain_id = chain_to_u64(p.chain());
        let amount_in = proto_to_biguint(&p.amount_in)?;

        let hops = p
            .hops
            .into_iter()
            .map(|h| req::RouteHop {
                component_id: h.component_id,
                token_in: h.token_in,
                token_out: h.token_out,
            })
            .collect();

        let mut state_overrides = std::collections::HashMap::new();
        for (k, v) in p.pool_state_overrides {
            if let Some(uuid) = bytes_to_uuid(&v) {
                state_overrides.insert(k, uuid);
            }
        }

        Ok(Self {
            chain_id,
            hops,
            amount_in,
            state_overrides,
        })
    }
}

impl TryFrom<proto::BatchGetAmountOutRequest> for req::BatchGetAmountOutRequest {
    type Error = BridgeError;

    fn try_from(p: proto::BatchGetAmountOutRequest) -> Result<Self, Self::Error> {
        let requests = p
            .requests
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { requests })
    }
}

impl From<proto::GetTokensRequest> for req::GetTokensRequest {
    fn from(p: proto::GetTokensRequest) -> Self {
        let chain = p.chain();
        let chain_id = chain_to_u64(chain);
        Self {
            chain_id: if chain_id > 0 { Some(chain_id) } else { None },
            addresses: Vec::new(), // Proto doesn't have addresses filter, use empty
            limit: if p.page_size > 0 {
                Some(p.page_size)
            } else {
                None
            },
        }
    }
}

impl From<proto::GetProtocolComponentsRequest> for req::GetProtocolComponentsRequest {
    fn from(p: proto::GetProtocolComponentsRequest) -> Self {
        Self {
            chain_id: chain_to_u64(p.chain()),
            component_ids: p.component_ids,
            min_tvl: p.min_tvl,
            protocol_type: p.protocol_systems.first().cloned(),
            limit: if p.page_size > 0 {
                Some(p.page_size)
            } else {
                None
            },
        }
    }
}

impl From<proto::GetStatusRequest> for req::GetStatusRequest {
    fn from(_p: proto::GetStatusRequest) -> Self {
        // GetStatusRequest has no fields in proto
        Self {
            chain_id: 0, // Will return status for all chains
            include_chain_details: true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Response Conversions: Core → Proto
// ─────────────────────────────────────────────────────────────────────────────

impl From<resp::SpotPriceResponse> for proto::SpotPriceResponse {
    fn from(r: resp::SpotPriceResponse) -> Self {
        // Calculate price as f64 for convenience
        let price = if !r.denominator.is_zero() {
            let num_f64 = r.numerator.to_string().parse::<f64>().unwrap_or(0.0);
            let denom_f64 = r.denominator.to_string().parse::<f64>().unwrap_or(1.0);
            num_f64 / denom_f64
        } else {
            0.0
        };

        Self {
            price,
            price_rational: Some(to_proto_price(&r.numerator, &r.denominator)),
        }
    }
}

impl From<resp::GetAmountOutResponse> for proto::GetAmountOutResponse {
    fn from(r: resp::GetAmountOutResponse) -> Self {
        Self {
            amount: Some(biguint_to_proto(&r.amount_out)),
            gas: Some(biguint_to_proto(&r.gas_cost)),
            new_pool_state: r.state_id.map(|id| uuid_to_bytes(&id)).unwrap_or_default(),
            execution_time_us: 0,  // TODO: Track execution time
            effective_price: 0.0,  // TODO: Calculate effective price
            price_impact_bps: 0.0, // TODO: Calculate price impact
            simulation_time_us: 0, // TODO: Track simulation time
        }
    }
}

impl From<resp::GetFeeResponse> for proto::GetFeeResponse {
    fn from(r: resp::GetFeeResponse) -> Self {
        // Calculate fee as f64 ratio
        let fee = if !r.fee_denominator.is_zero() {
            let num_f64 = r.fee_numerator.to_string().parse::<f64>().unwrap_or(0.0);
            let denom_f64 = r.fee_denominator.to_string().parse::<f64>().unwrap_or(1.0);
            num_f64 / denom_f64
        } else {
            0.0
        };

        // Convert to basis points (multiply by 10000)
        let fee_bps = (fee * 10000.0) as u32;

        Self {
            fee,
            fee_bps,
            is_dynamic: false, // TODO: Track if fee is dynamic
        }
    }
}

impl From<resp::GetLimitsResponse> for proto::GetLimitsResponse {
    fn from(r: resp::GetLimitsResponse) -> Self {
        Self {
            max_sell_amount: Some(biguint_to_proto(&r.max_amount_in)),
            max_buy_amount: Some(biguint_to_proto(&r.min_amount_in)), // Note: min_amount_in maps to max_buy
            is_soft_limit: true,                                      // Most limits are soft limits
        }
    }
}

impl From<resp::TradeInfo> for proto::Trade {
    fn from(t: resp::TradeInfo) -> Self {
        Self {
            amount_in: Some(biguint_to_proto(&t.amount_in)),
            amount_out: Some(biguint_to_proto(&t.amount_out)),
            // Note: proto Trade doesn't have a price field
        }
    }
}

impl From<resp::SwapToPriceResponse> for proto::SwapToPriceResponse {
    fn from(r: resp::SwapToPriceResponse) -> Self {
        Self {
            trade: r.trade.map(|t| t.into()),
            is_reachable: r.reached_target,
        }
    }
}

impl From<resp::QuerySupplyResponse> for proto::QuerySupplyResponse {
    fn from(r: resp::QuerySupplyResponse) -> Self {
        // Proto expects a single trade, take the first one if available
        Self {
            trade: r.trades.into_iter().next().map(|t| t.into()),
        }
    }
}

impl From<resp::HopResult> for proto::HopResult {
    fn from(h: resp::HopResult) -> Self {
        Self {
            component_id: h.component_id,
            amount_in: None, // Core HopResult doesn't track amount_in per hop
            amount_out: Some(biguint_to_proto(&h.amount_out)),
            gas: Some(biguint_to_proto(&h.gas_cost)),
            effective_price: 0.0,  // TODO: Calculate effective price
            price_impact_bps: 0.0, // TODO: Calculate price impact
        }
    }
}

impl From<resp::SimulateRouteResponse> for proto::SimulateRouteResponse {
    fn from(r: resp::SimulateRouteResponse) -> Self {
        Self {
            amount_out: Some(biguint_to_proto(&r.final_amount_out)),
            total_gas: Some(biguint_to_proto(&r.total_gas_cost)),
            hops: r.hop_results.into_iter().map(|h| h.into()).collect(),
            final_pool_states: std::collections::HashMap::new(), // TODO: Populate from core response
            total_price_impact_bps: 0.0, // TODO: Calculate total price impact
            total_execution_time_us: 0,  // TODO: Track execution time
        }
    }
}

impl From<resp::BatchGetAmountOutResponse> for proto::BatchGetAmountOutResponse {
    fn from(r: resp::BatchGetAmountOutResponse) -> Self {
        use crate::proto::get_amount_out_result::Result as ProtoResult;

        let results = r
            .results
            .into_iter()
            .map(|opt| proto::GetAmountOutResult {
                result: opt.map(|resp| ProtoResult::Success(resp.into())),
            })
            .collect();

        Self {
            results,
            total_execution_time_us: 0, // TODO: Track execution time
        }
    }
}

impl From<resp::TokenInfo> for proto::Token {
    fn from(t: resp::TokenInfo) -> Self {
        Self {
            address: t.address,
            symbol: t.symbol,
            decimals: t.decimals,
            quality: 100, // Default to normal token quality
        }
    }
}

impl From<resp::GetTokensResponse> for proto::GetTokensResponse {
    fn from(r: resp::GetTokensResponse) -> Self {
        let tokens: Vec<_> = r.tokens.into_iter().map(|t| t.into()).collect();
        let total_count = tokens.len() as u32;
        Self {
            tokens,
            total_count,
        }
    }
}

// Note: resp::SyncStatus is an enum (Unknown, Syncing, Synced, Error)
// but proto::ProtocolSyncStatus is a message with string status.
// The conversion is handled directly in the GetStatusResponse conversion.

impl From<resp::GetStatusResponse> for proto::GetStatusResponse {
    fn from(r: resp::GetStatusResponse) -> Self {
        // Note: The proto GetStatusResponse has different fields than the core response
        // We convert what we can and leave the rest as defaults
        Self {
            healthy: r.sync_status == resp::SyncStatus::Synced,
            version: env!("CARGO_PKG_VERSION").to_string(),
            chains: Vec::new(), // TODO: Build ChainStatus from core response
            uptime_seconds: 0,  // TODO: Track uptime in BridgeService
        }
    }
}

impl From<resp::EncodeSolutionResponse> for proto::EncodeSolutionResponse {
    fn from(r: resp::EncodeSolutionResponse) -> Self {
        Self {
            encoded_swaps: r.calldata,
            interacting_with: r.router_address,
            selector: String::new(), // TODO: Extract selector from calldata
            n_tokens: 0,             // TODO: Count tokens if needed
            permit: None,
            full_calldata: None, // We don't build full calldata (not recommended for production)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Error Conversions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert BridgeError to tonic::Status.
///
/// Note: We can't implement `From<BridgeError> for tonic::Status` due to orphan rules.
/// Both types are from external crates, so we use a standalone function instead.
pub fn bridge_error_to_status(e: BridgeError) -> tonic::Status {
    match &e {
        BridgeError::ComponentNotFound { .. } => tonic::Status::not_found(e.to_string()),
        BridgeError::TokenNotFound { .. } => tonic::Status::not_found(e.to_string()),
        BridgeError::InvalidArgument { .. } => tonic::Status::invalid_argument(e.to_string()),
        BridgeError::SimulationError { .. } => tonic::Status::internal(e.to_string()),
        BridgeError::StateUnavailable { .. } => tonic::Status::unavailable(e.to_string()),
        BridgeError::ChainNotSupported { .. } => tonic::Status::invalid_argument(e.to_string()),
        BridgeError::ConfigError { .. } => tonic::Status::internal(e.to_string()),
        BridgeError::ConnectionError { .. } => tonic::Status::unavailable(e.to_string()),
        BridgeError::Internal { .. } => tonic::Status::internal(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // Chain Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_chain_to_u64() {
        assert_eq!(chain_to_u64(Chain::Ethereum), 1);
        assert_eq!(chain_to_u64(Chain::Base), 8453);
        assert_eq!(chain_to_u64(Chain::Unichain), 130);
        assert_eq!(chain_to_u64(Chain::Arbitrum), 42161);
        assert_eq!(chain_to_u64(Chain::Unspecified), 0);
    }

    #[test]
    fn test_u64_to_chain() {
        assert_eq!(u64_to_chain(1), Chain::Ethereum);
        assert_eq!(u64_to_chain(8453), Chain::Base);
        assert_eq!(u64_to_chain(130), Chain::Unichain);
        assert_eq!(u64_to_chain(42161), Chain::Arbitrum);
        assert_eq!(u64_to_chain(999), Chain::Unspecified); // Unknown chain
    }

    #[test]
    fn test_chain_roundtrip() {
        for chain in [Chain::Ethereum, Chain::Base, Chain::Unichain, Chain::Arbitrum] {
            let id = chain_to_u64(chain);
            assert_eq!(u64_to_chain(id), chain);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // BigInt Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_biguint_to_proto() {
        let value = BigUint::from(12345u64);
        let proto = biguint_to_proto(&value);
        assert_eq!(proto.value, "12345");
    }

    #[test]
    fn test_biguint_to_proto_zero() {
        let value = BigUint::zero();
        let proto = biguint_to_proto(&value);
        assert_eq!(proto.value, "0");
    }

    #[test]
    fn test_proto_to_biguint_success() {
        let proto = Some(BigInt {
            value: "12345".to_string(),
        });
        let result = proto_to_biguint(&proto).unwrap();
        assert_eq!(result, BigUint::from(12345u64));
    }

    #[test]
    fn test_proto_to_biguint_empty_value() {
        let proto = Some(BigInt {
            value: String::new(),
        });
        let result = proto_to_biguint(&proto).unwrap();
        assert_eq!(result, BigUint::zero());
    }

    #[test]
    fn test_proto_to_biguint_missing() {
        let result = proto_to_biguint(&None);
        assert!(result.is_err());
    }

    #[test]
    fn test_proto_to_biguint_invalid() {
        let proto = Some(BigInt {
            value: "not_a_number".to_string(),
        });
        let result = proto_to_biguint(&proto);
        assert!(result.is_err());
    }

    #[test]
    fn test_proto_to_biguint_or_zero_missing() {
        let result = proto_to_biguint_or_zero(&None);
        assert_eq!(result, BigUint::zero());
    }

    #[test]
    fn test_proto_to_biguint_or_zero_invalid() {
        let proto = Some(BigInt {
            value: "invalid".to_string(),
        });
        let result = proto_to_biguint_or_zero(&proto);
        assert_eq!(result, BigUint::zero());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UUID Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_uuid_to_bytes() {
        let uuid = Uuid::nil();
        let bytes = uuid_to_bytes(&uuid);
        assert_eq!(bytes.len(), 16);
        assert!(bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_bytes_to_uuid_valid() {
        let original = Uuid::new_v4();
        let bytes = uuid_to_bytes(&original);
        let recovered = bytes_to_uuid(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_bytes_to_uuid_wrong_length() {
        assert!(bytes_to_uuid(&[0u8; 15]).is_none()); // Too short
        assert!(bytes_to_uuid(&[0u8; 17]).is_none()); // Too long
        assert!(bytes_to_uuid(&[]).is_none());        // Empty
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Price Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_to_proto_price() {
        let num = BigUint::from(3u64);
        let denom = BigUint::from(2u64);
        let price = to_proto_price(&num, &denom);

        assert_eq!(price.numerator.as_ref().unwrap().value, "3");
        assert_eq!(price.denominator.as_ref().unwrap().value, "2");
    }

    #[test]
    fn test_f64_to_proto_price() {
        let price = f64_to_proto_price(1.5);
        // Should be approximately 3/2 or equivalent
        let num: f64 = price.numerator.as_ref().unwrap().value.parse().unwrap();
        let denom: f64 = price.denominator.as_ref().unwrap().value.parse().unwrap();
        assert!((num / denom - 1.5).abs() < 0.0001);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Error Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_bridge_error_to_status_not_found() {
        let err = BridgeError::component_not_found("pool_123");
        let status = bridge_error_to_status(err);
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[test]
    fn test_bridge_error_to_status_invalid_argument() {
        let err = BridgeError::invalid_argument("bad input");
        let status = bridge_error_to_status(err);
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn test_bridge_error_to_status_unavailable() {
        let err = BridgeError::StateUnavailable {
            message: "not synced".to_string(),
        };
        let status = bridge_error_to_status(err);
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn test_bridge_error_to_status_internal() {
        let err = BridgeError::internal("oops");
        let status = bridge_error_to_status(err);
        assert_eq!(status.code(), tonic::Code::Internal);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Request Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_spot_price_request_conversion() {
        let proto_req = proto::SpotPriceRequest {
            chain: Chain::Ethereum.into(),
            component_id: "pool_1".to_string(),
            base_token: "0xtoken_a".to_string(),
            quote_token: "0xtoken_b".to_string(),
        };

        let core_req: req::SpotPriceRequest = proto_req.try_into().unwrap();
        assert_eq!(core_req.chain_id, 1);
        assert_eq!(core_req.component_id, "pool_1");
    }

    #[test]
    fn test_get_amount_out_request_conversion() {
        let proto_req = proto::GetAmountOutRequest {
            chain: Chain::Base.into(),
            component_id: "pool_1".to_string(),
            token_in: "0xa".to_string(),
            token_out: "0xb".to_string(),
            amount_in: Some(BigInt {
                value: "1000".to_string(),
            }),
            pool_state_override: None,
        };

        let core_req: req::GetAmountOutRequest = proto_req.try_into().unwrap();
        assert_eq!(core_req.chain_id, 8453);
        assert_eq!(core_req.amount_in, BigUint::from(1000u64));
        assert!(core_req.save_state); // Always true
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Response Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_spot_price_response_conversion() {
        let core_resp = resp::SpotPriceResponse {
            numerator: BigUint::from(3u64),
            denominator: BigUint::from(2u64),
        };

        let proto_resp: proto::SpotPriceResponse = core_resp.into();
        assert!((proto_resp.price - 1.5).abs() < 0.0001);
        assert!(proto_resp.price_rational.is_some());
    }

    #[test]
    fn test_get_amount_out_response_conversion() {
        let state_id = Uuid::new_v4();
        let core_resp = resp::GetAmountOutResponse {
            amount_out: BigUint::from(1000u64),
            gas_cost: BigUint::from(50000u64),
            state_id: Some(state_id),
            execution_price: None,
        };

        let proto_resp: proto::GetAmountOutResponse = core_resp.into();
        assert_eq!(proto_resp.amount.as_ref().unwrap().value, "1000");
        assert_eq!(proto_resp.gas.as_ref().unwrap().value, "50000");
        assert_eq!(proto_resp.new_pool_state.len(), 16); // UUID bytes
    }

    #[test]
    fn test_get_fee_response_conversion() {
        let core_resp = resp::GetFeeResponse {
            fee_numerator: BigUint::from(3u64),
            fee_denominator: BigUint::from(1000u64),
        };

        let proto_resp: proto::GetFeeResponse = core_resp.into();
        assert!((proto_resp.fee - 0.003).abs() < 0.0001);
        assert_eq!(proto_resp.fee_bps, 30); // 0.003 * 10000 = 30
    }
}
