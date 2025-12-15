//! gRPC service implementation.
//!
//! This module contains the tonic service that wraps BridgeService.
//! The GrpcService is a thin adapter that:
//! 1. Converts proto request types to tycho-core native types
//! 2. Calls the appropriate BridgeService handler
//! 3. Converts the native response back to proto types

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, instrument};

use tycho_core::BridgeService;

use crate::convert::{self, bridge_error_to_status};
use crate::proto::{
    tycho_bridge_server::TychoBridge, BatchGetAmountOutRequest, BatchGetAmountOutResponse,
    EncodeSolutionRequest, EncodeSolutionResponse, GetAmountOutRequest, GetAmountOutResponse,
    GetFeeRequest, GetFeeResponse, GetLimitsRequest, GetLimitsResponse,
    GetProtocolComponentsRequest, GetProtocolComponentsResponse, GetStatusRequest,
    GetStatusResponse, GetTokensRequest, GetTokensResponse, ProtoEncoderType, ProtoNativeAction,
    ProtoUserTransferType, ProtocolComponent, QuerySupplyRequest, QuerySupplyResponse,
    SimulateRouteRequest, SimulateRouteResponse, SpotPriceRequest, SpotPriceResponse,
    StreamUpdatesRequest, SwapToPriceRequest, SwapToPriceResponse, Update,
};

/// gRPC service wrapping BridgeService.
///
/// This is a thin adapter layer that handles proto ↔ native type conversions
/// and delegates all business logic to the underlying BridgeService.
pub struct GrpcService {
    bridge: Arc<BridgeService>,
}

impl GrpcService {
    /// Create a new GrpcService wrapping a BridgeService.
    pub fn new(bridge: Arc<BridgeService>) -> Self {
        Self { bridge }
    }
}

// Type alias for streaming response
type UpdateStream = Pin<Box<dyn Stream<Item = Result<Update, Status>> + Send>>;

#[tonic::async_trait]
impl TychoBridge for GrpcService {
    // ─────────────────────────────────────────────────────────────────────────
    // Simulation Methods
    // ─────────────────────────────────────────────────────────────────────────

    #[instrument(skip(self, request), fields(method = "SpotPrice"))]
    async fn spot_price(
        &self,
        request: Request<SpotPriceRequest>,
    ) -> Result<Response<SpotPriceResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(component_id = %proto_req.component_id, "SpotPrice request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .spot_price(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "GetAmountOut"))]
    async fn get_amount_out(
        &self,
        request: Request<GetAmountOutRequest>,
    ) -> Result<Response<GetAmountOutResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(component_id = %proto_req.component_id, "GetAmountOut request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .get_amount_out(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "GetFee"))]
    async fn get_fee(
        &self,
        request: Request<GetFeeRequest>,
    ) -> Result<Response<GetFeeResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(component_id = %proto_req.component_id, "GetFee request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .get_fee(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "GetLimits"))]
    async fn get_limits(
        &self,
        request: Request<GetLimitsRequest>,
    ) -> Result<Response<GetLimitsResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(component_id = %proto_req.component_id, "GetLimits request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .get_limits(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "SwapToPrice"))]
    async fn swap_to_price(
        &self,
        request: Request<SwapToPriceRequest>,
    ) -> Result<Response<SwapToPriceResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(component_id = %proto_req.component_id, "SwapToPrice request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .swap_to_price(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "QuerySupply"))]
    async fn query_supply(
        &self,
        request: Request<QuerySupplyRequest>,
    ) -> Result<Response<QuerySupplyResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(component_id = %proto_req.component_id, "QuerySupply request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .query_supply(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "BatchGetAmountOut"))]
    async fn batch_get_amount_out(
        &self,
        request: Request<BatchGetAmountOutRequest>,
    ) -> Result<Response<BatchGetAmountOutResponse>, Status> {
        let proto_req = request.into_inner();
        let request_count = proto_req.requests.len();
        debug!(request_count, "BatchGetAmountOut request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .batch_get_amount_out(core_req)
            .map_err(bridge_error_to_status)?;

        info!(request_count, "BatchGetAmountOut completed");
        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "SimulateRoute"))]
    async fn simulate_route(
        &self,
        request: Request<SimulateRouteRequest>,
    ) -> Result<Response<SimulateRouteResponse>, Status> {
        let proto_req = request.into_inner();
        let hop_count = proto_req.hops.len();
        debug!(hop_count, "SimulateRoute request");

        let core_req = proto_req.try_into().map_err(bridge_error_to_status)?;
        let core_resp = self
            .bridge
            .simulate_route(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Indexing Methods
    // ─────────────────────────────────────────────────────────────────────────

    #[instrument(skip(self, request), fields(method = "GetTokens"))]
    async fn get_tokens(
        &self,
        request: Request<GetTokensRequest>,
    ) -> Result<Response<GetTokensResponse>, Status> {
        let proto_req = request.into_inner();
        debug!("GetTokens request");

        let core_req = proto_req.into();
        let core_resp = self
            .bridge
            .get_tokens_async(core_req)
            .await
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    #[instrument(skip(self, request), fields(method = "GetProtocolComponents"))]
    async fn get_protocol_components(
        &self,
        request: Request<GetProtocolComponentsRequest>,
    ) -> Result<Response<GetProtocolComponentsResponse>, Status> {
        let proto_req = request.into_inner();
        debug!(chain = ?convert::chain_to_u64(proto_req.chain()), "GetProtocolComponents request");

        let core_req = proto_req.into();
        let core_resp = self
            .bridge
            .get_components(core_req)
            .map_err(bridge_error_to_status)?;

        // Convert core response to proto
        let total_count = core_resp.components.len() as u32;
        let proto_resp = GetProtocolComponentsResponse {
            components: core_resp
                .components
                .into_iter()
                .map(|c| {
                    let chain_id = c.chain_id;
                    ProtocolComponent {
                        id: c.id,
                        protocol_system: c.protocol_system,
                        protocol_type_name: c.protocol_type_name,
                        chain: convert::u64_to_chain(chain_id).into(),
                        tokens: c.tokens,               // Already Vec<String>
                        contract_addresses: Vec::new(), // TODO: Include if available
                        static_attributes: std::collections::HashMap::new(), // TODO: Include if needed
                        creation_tx: String::new(), // TODO: Include if available
                    }
                })
                .collect(),
            total_count,
        };

        Ok(Response::new(proto_resp))
    }

    type StreamUpdatesStream = UpdateStream;

    #[instrument(skip(self, request), fields(method = "StreamUpdates"))]
    async fn stream_updates(
        &self,
        request: Request<StreamUpdatesRequest>,
    ) -> Result<Response<Self::StreamUpdatesStream>, Status> {
        let _proto_req = request.into_inner();
        debug!("StreamUpdates request - not yet implemented");

        // TODO: Implement streaming updates from TychoManager
        // For now, return an empty stream
        error!("StreamUpdates not yet implemented in tycho-grpc");

        let stream = tokio_stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Execution Methods
    // ─────────────────────────────────────────────────────────────────────────

    #[instrument(skip(self, request), fields(method = "EncodeSolution"))]
    async fn encode_solution(
        &self,
        request: Request<EncodeSolutionRequest>,
    ) -> Result<Response<EncodeSolutionResponse>, Status> {
        let proto_req = request.into_inner();
        debug!("EncodeSolution request");

        // Convert proto request to core request
        // This is more complex because of the nested Solution structure
        let core_req = convert_encode_solution_request(proto_req)?;
        let core_resp = self
            .bridge
            .encode_solution(core_req)
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Status Methods
    // ─────────────────────────────────────────────────────────────────────────

    #[instrument(skip(self, request), fields(method = "GetStatus"))]
    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let proto_req = request.into_inner();
        debug!("GetStatus request");

        let core_req = proto_req.into();
        let core_resp = self
            .bridge
            .get_status_async(core_req)
            .await
            .map_err(bridge_error_to_status)?;

        Ok(Response::new(core_resp.into()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Complex Conversion Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert proto EncodeSolutionRequest to core request.
fn convert_encode_solution_request(
    p: EncodeSolutionRequest,
) -> Result<tycho_core::types::request::EncodeSolutionRequest, Status> {
    use tycho_core::types::request::{
        ComponentInfo, EncodeSolutionRequest as CoreEncodeSolutionRequest, EncoderType,
        NativeAction, Permit, SwapInfo, UserTransferType,
    };

    let chain_id = convert::chain_to_u64(p.chain());
    let encoder_type_proto = p.encoder_type;
    let user_transfer_type_proto = p.user_transfer_type;

    let solution = p
        .solution
        .ok_or_else(|| Status::invalid_argument("Missing solution"))?;

    // Convert swaps
    let swaps = solution
        .swaps
        .into_iter()
        .map(|s| {
            Ok(SwapInfo {
                component_id: s
                    .component
                    .as_ref()
                    .map(|c| c.id.clone())
                    .unwrap_or_default(),
                token_in: s.token_in.clone(),
                token_out: s.token_out.clone(),
                amount_in: convert::proto_to_biguint_or_zero(&s.estimated_amount_in),
                min_amount_out: num_bigint::BigUint::ZERO, // Not in proto Swap, set default
                component: s.component.map(|c| ComponentInfo {
                    id: c.id,
                    protocol_system: c.protocol_system,
                    tokens: c.tokens,
                    static_attributes: Vec::new(), // TODO: Convert attributes
                }),
            })
        })
        .collect::<Result<Vec<_>, Status>>()?;

    // Convert encoder type
    let encoder_type = match ProtoEncoderType::try_from(encoder_type_proto) {
        Ok(ProtoEncoderType::Unspecified) | Ok(ProtoEncoderType::TychoRouter) => {
            EncoderType::TychoRouter
        }
        Ok(ProtoEncoderType::TychoExecutor) => EncoderType::TychoRouter, // Map executor to router for now
        Err(_) => EncoderType::TychoRouter,
    };

    // Convert user transfer type
    let user_transfer_type = match ProtoUserTransferType::try_from(user_transfer_type_proto) {
        Ok(ProtoUserTransferType::TransferFrom) => UserTransferType::TransferFrom,
        Ok(ProtoUserTransferType::TransferFromPermit2) => UserTransferType::Permit2,
        Ok(ProtoUserTransferType::None) => UserTransferType::None,
        _ => UserTransferType::TransferFrom,
    };

    // Convert native action
    let native_action = match solution.native_action {
        Some(n) => match ProtoNativeAction::try_from(n) {
            Ok(ProtoNativeAction::Unspecified) => NativeAction::None,
            Ok(ProtoNativeAction::Wrap) => NativeAction::Wrap,
            Ok(ProtoNativeAction::Unwrap) => NativeAction::Unwrap,
            Err(_) => NativeAction::None,
        },
        None => NativeAction::None,
    };

    // No permit in proto Solution struct
    let permit: Option<Permit> = None;

    // Determine given_is_input based on exact_out
    let given_is_input = !solution.exact_out;

    Ok(CoreEncodeSolutionRequest {
        chain_id,
        sender: solution.sender,
        receiver: solution.receiver,
        given_token: solution.given_token,
        given_amount: convert::proto_to_biguint_or_zero(&solution.given_amount),
        given_is_input,
        exact_out_amount: if solution.exact_out {
            Some(convert::proto_to_biguint_or_zero(&solution.checked_amount))
        } else {
            None
        },
        checked_token: solution.checked_token,
        expected_amount: convert::proto_to_biguint_or_zero(&solution.checked_amount),
        is_expected_min: given_is_input, // If input is given, expected is min output
        swaps,
        user_transfer_type,
        native_action,
        encoder_type,
        permit,
        router_address: if solution.router_address.is_empty() {
            None
        } else {
            Some(solution.router_address)
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{BigInt, Solution, Swap};

    #[test]
    fn test_convert_encode_solution_request_missing_solution() {
        let proto_req = EncodeSolutionRequest {
            chain: 1, // Ethereum
            solution: None,
            encoder_type: 0,
            user_transfer_type: 0,
        };

        let result = convert_encode_solution_request(proto_req);
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn test_convert_encode_solution_request_basic() {
        let proto_req = EncodeSolutionRequest {
            chain: 1, // Ethereum
            solution: Some(Solution {
                sender: "0xsender".to_string(),
                receiver: "0xreceiver".to_string(),
                given_token: "0xtoken_in".to_string(),
                given_amount: Some(BigInt {
                    value: "1000".to_string(),
                }),
                checked_token: "0xtoken_out".to_string(),
                checked_amount: Some(BigInt {
                    value: "900".to_string(),
                }),
                exact_out: false,
                swaps: vec![],
                native_action: None,
                router_address: String::new(),
            }),
            encoder_type: ProtoEncoderType::TychoRouter.into(),
            user_transfer_type: ProtoUserTransferType::TransferFrom.into(),
        };

        let result = convert_encode_solution_request(proto_req).unwrap();
        assert_eq!(result.chain_id, 1);
        assert_eq!(result.sender, "0xsender");
        assert_eq!(result.receiver, "0xreceiver");
        assert!(result.given_is_input); // exact_out = false means given is input
        assert!(result.router_address.is_none()); // Empty string -> None
    }

    #[test]
    fn test_convert_encode_solution_request_with_router_address() {
        let proto_req = EncodeSolutionRequest {
            chain: 8453, // Base
            solution: Some(Solution {
                sender: "0xsender".to_string(),
                receiver: "0xreceiver".to_string(),
                given_token: "0xtoken".to_string(),
                given_amount: Some(BigInt {
                    value: "100".to_string(),
                }),
                checked_token: "0xout".to_string(),
                checked_amount: Some(BigInt {
                    value: "90".to_string(),
                }),
                exact_out: false,
                swaps: vec![],
                native_action: None,
                router_address: "0xcustom_router".to_string(),
            }),
            encoder_type: 0,
            user_transfer_type: 0,
        };

        let result = convert_encode_solution_request(proto_req).unwrap();
        assert_eq!(result.router_address, Some("0xcustom_router".to_string()));
    }

    #[test]
    fn test_convert_encode_solution_request_exact_out() {
        let proto_req = EncodeSolutionRequest {
            chain: 1,
            solution: Some(Solution {
                sender: "0xsender".to_string(),
                receiver: "0xreceiver".to_string(),
                given_token: "0xtoken".to_string(),
                given_amount: Some(BigInt {
                    value: "1000".to_string(),
                }),
                checked_token: "0xout".to_string(),
                checked_amount: Some(BigInt {
                    value: "500".to_string(),
                }),
                exact_out: true,
                swaps: vec![],
                native_action: None,
                router_address: String::new(),
            }),
            encoder_type: 0,
            user_transfer_type: 0,
        };

        let result = convert_encode_solution_request(proto_req).unwrap();
        assert!(!result.given_is_input); // exact_out = true means given is output
        assert!(result.exact_out_amount.is_some());
    }

    #[test]
    fn test_convert_encode_solution_request_with_swaps() {
        let proto_req = EncodeSolutionRequest {
            chain: 1,
            solution: Some(Solution {
                sender: "0xsender".to_string(),
                receiver: "0xreceiver".to_string(),
                given_token: "0xtoken_a".to_string(),
                given_amount: Some(BigInt {
                    value: "1000".to_string(),
                }),
                checked_token: "0xtoken_c".to_string(),
                checked_amount: Some(BigInt {
                    value: "800".to_string(),
                }),
                exact_out: false,
                swaps: vec![
                    Swap {
                        component: Some(ProtocolComponent {
                            id: "pool_1".to_string(),
                            protocol_system: "uniswap_v2".to_string(),
                            protocol_type_name: "pool".to_string(),
                            chain: 1,
                            tokens: vec!["0xtoken_a".to_string(), "0xtoken_b".to_string()],
                            contract_addresses: vec![],
                            static_attributes: std::collections::HashMap::new(),
                            creation_tx: String::new(),
                        }),
                        token_in: "0xtoken_a".to_string(),
                        token_out: "0xtoken_b".to_string(),
                        split: 1.0,
                        protocol_state: None,
                        estimated_amount_in: Some(BigInt {
                            value: "1000".to_string(),
                        }),
                    },
                ],
                native_action: None,
                router_address: String::new(),
            }),
            encoder_type: 0,
            user_transfer_type: 0,
        };

        let result = convert_encode_solution_request(proto_req).unwrap();
        assert_eq!(result.swaps.len(), 1);
        assert_eq!(result.swaps[0].component_id, "pool_1");
        assert_eq!(result.swaps[0].token_in, "0xtoken_a");
        assert_eq!(result.swaps[0].token_out, "0xtoken_b");
    }

    #[test]
    fn test_convert_encode_solution_native_actions() {
        // Test Wrap
        let proto_req = EncodeSolutionRequest {
            chain: 1,
            solution: Some(Solution {
                sender: "0xsender".to_string(),
                receiver: "0xreceiver".to_string(),
                given_token: "0xeth".to_string(),
                given_amount: Some(BigInt { value: "100".to_string() }),
                checked_token: "0xweth".to_string(),
                checked_amount: Some(BigInt { value: "100".to_string() }),
                exact_out: false,
                swaps: vec![],
                native_action: Some(ProtoNativeAction::Wrap.into()),
                router_address: String::new(),
            }),
            encoder_type: 0,
            user_transfer_type: 0,
        };

        let result = convert_encode_solution_request(proto_req).unwrap();
        assert_eq!(result.native_action, tycho_core::types::request::NativeAction::Wrap);
    }
}
