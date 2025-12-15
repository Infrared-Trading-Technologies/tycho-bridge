//! Execution handlers - Calldata encoding operations.
//!
//! This module implements:
//! - `encode_solution` - Encode a multi-swap solution for Tycho Router
//! - `encode_calldata` - Encode simple swap calldata
//! - `batch_encode_calldata` - Batch encode multiple swaps

use std::str::FromStr;

use num_bigint::BigUint;
use tracing::{debug, error, info, instrument, warn};

use crate::bridge::BridgeService;
use crate::domain::{
    Bytes, NativeAction as TychoNativeAction, Solution, SwapBuilder, TychoRouterEncoderBuilder,
    UserTransferType as TychoUserTransferType,
};
use crate::error::{BridgeError, BridgeResult};
use crate::helpers::chain_id_to_tycho_chain;
use crate::state::make_composite_key;
use crate::types::request::{
    BatchEncodeCalldataRequest, EncodeCalldataRequest, EncodeSolutionRequest, NativeAction,
    SwapInfo, UserTransferType,
};
use crate::types::response::{
    BatchEncodeCalldataResponse, EncodeCalldataResponse, EncodeSolutionResponse,
};

impl BridgeService {
    /// Encode a complete solution for the Tycho Router.
    ///
    /// This takes a multi-hop swap solution and encodes it into calldata
    /// that can be submitted to the Tycho Router contract on-chain.
    #[instrument(skip(self), fields(chain_id = req.chain_id, swaps = req.swaps.len()))]
    pub fn encode_solution(
        &self,
        req: EncodeSolutionRequest,
    ) -> BridgeResult<EncodeSolutionResponse> {
        self.validate_chain(req.chain_id)?;

        let chain = chain_id_to_tycho_chain(req.chain_id)?;

        // Parse addresses
        let sender = parse_address(&req.sender)?;
        let receiver = parse_address(&req.receiver)?;
        let given_token = parse_address(&req.given_token)?;
        let checked_token = parse_address(&req.checked_token)?;

        // Convert swaps
        let mut swaps = Vec::with_capacity(req.swaps.len());
        for (idx, swap_info) in req.swaps.iter().enumerate() {
            let swap = self.convert_swap_info_to_tycho(req.chain_id, swap_info, idx)?;
            swaps.push(swap);
        }

        if swaps.is_empty() {
            return Err(BridgeError::invalid_argument(
                "Solution must contain at least one swap",
            ));
        }

        // Convert native action
        let native_action = match req.native_action {
            NativeAction::Wrap => Some(TychoNativeAction::Wrap),
            NativeAction::Unwrap => Some(TychoNativeAction::Unwrap),
            NativeAction::None => None,
        };

        // Determine if exact_out based on the given_is_input flag
        let exact_out = !req.given_is_input;

        // Calculate checked_amount (expected output for exact_in, expected input for exact_out)
        let checked_amount = req.expected_amount.clone();

        // Build the tycho-execution Solution
        let solution = Solution {
            sender: sender.clone(),
            receiver: receiver.clone(),
            given_token,
            given_amount: req.given_amount.clone(),
            checked_token,
            checked_amount,
            exact_out,
            swaps,
            native_action,
        };

        // Convert user transfer type
        let user_transfer_type = match req.user_transfer_type {
            UserTransferType::TransferFrom => TychoUserTransferType::TransferFrom,
            UserTransferType::Permit2 => TychoUserTransferType::TransferFromPermit2,
            UserTransferType::None => TychoUserTransferType::None,
        };

        // Build the encoder
        let encoder = TychoRouterEncoderBuilder::new()
            .chain(chain)
            .user_transfer_type(user_transfer_type)
            .build()
            .map_err(|e| {
                error!(error = %e, "Failed to build encoder");
                BridgeError::internal(format!("Failed to build encoder: {}", e))
            })?;

        // Encode the solution
        let encoded_solutions = encoder.encode_solutions(vec![solution]).map_err(|e| {
            warn!(error = %e, "Encoding failed");
            BridgeError::internal(format!("Encoding failed: {}", e))
        })?;

        let encoded = encoded_solutions
            .first()
            .ok_or_else(|| BridgeError::internal("No encoded solution returned"))?;

        // Extract full calldata
        let calldata = encoded.swaps.clone();
        let router_address = format!("0x{}", hex::encode(&encoded.interacting_with));

        // For value, we need to check if we're wrapping native ETH
        let value = if req.native_action == NativeAction::Wrap {
            req.given_amount.clone()
        } else {
            BigUint::ZERO
        };

        info!(
            router = %router_address,
            calldata_len = calldata.len(),
            "EncodeSolution completed"
        );

        Ok(EncodeSolutionResponse {
            calldata,
            router_address,
            value,
        })
    }

    /// Encode calldata for a simple single swap execution.
    ///
    /// This is a simplified interface for single-swap scenarios.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn encode_calldata(
        &self,
        req: EncodeCalldataRequest,
    ) -> BridgeResult<EncodeCalldataResponse> {
        self.validate_chain(req.chain_id)?;

        // Look up component to get protocol information
        let composite_key = make_composite_key(req.chain_id, &req.component_id);
        let component = self
            .state_cache
            .get_component(&composite_key)
            .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?;

        // Build single swap
        let token_in = parse_address(&req.token_in)?;
        let token_out = parse_address(&req.token_out)?;

        let swap = SwapBuilder::new(component, token_in.clone(), token_out.clone())
            .split(1.0) // 100% of input
            .build();

        // Parse recipient
        let recipient = parse_address(&req.recipient)?;

        // Build solution with single swap
        let solution = Solution {
            sender: recipient.clone(),
            receiver: recipient.clone(),
            given_token: token_in,
            given_amount: req.amount_in.clone(),
            checked_token: token_out,
            checked_amount: req.min_amount_out.clone(),
            exact_out: false,
            swaps: vec![swap],
            native_action: None,
        };

        // Get chain
        let chain = chain_id_to_tycho_chain(req.chain_id)?;

        // Build encoder with TransferFrom (typical case)
        let encoder = TychoRouterEncoderBuilder::new()
            .chain(chain)
            .user_transfer_type(TychoUserTransferType::TransferFrom)
            .build()
            .map_err(|e| {
                error!(error = %e, "Failed to build encoder");
                BridgeError::internal(format!("Failed to build encoder: {}", e))
            })?;

        // Encode
        let encoded_solutions = encoder.encode_solutions(vec![solution]).map_err(|e| {
            warn!(error = %e, "Encoding failed");
            BridgeError::internal(format!("Encoding failed: {}", e))
        })?;

        let encoded = encoded_solutions
            .first()
            .ok_or_else(|| BridgeError::internal("No encoded solution returned"))?;

        debug!(
            component_id = %req.component_id,
            calldata_len = encoded.swaps.len(),
            "Calldata encoded"
        );

        Ok(EncodeCalldataResponse {
            calldata: encoded.swaps.clone(),
            to: format!("0x{}", hex::encode(&encoded.interacting_with)),
            value: BigUint::ZERO,
        })
    }

    /// Batch encode calldata for multiple swaps.
    #[instrument(skip(self), fields(count = req.requests.len()))]
    pub fn batch_encode_calldata(
        &self,
        req: BatchEncodeCalldataRequest,
    ) -> BridgeResult<BatchEncodeCalldataResponse> {
        let mut results = Vec::with_capacity(req.requests.len());

        for single_req in req.requests {
            match self.encode_calldata(single_req) {
                Ok(resp) => results.push(Some(resp)),
                Err(e) => {
                    debug!(error = %e, "Batch encode single failure");
                    results.push(None);
                }
            }
        }

        Ok(BatchEncodeCalldataResponse { results })
    }

    /// Convert SwapInfo to tycho-execution Swap.
    fn convert_swap_info_to_tycho(
        &self,
        chain_id: u64,
        swap_info: &SwapInfo,
        idx: usize,
    ) -> BridgeResult<crate::domain::Swap> {
        // Get component from cache or from request
        let component = if let Some(ref comp_info) = swap_info.component {
            // Try cache first
            let composite_key = make_composite_key(chain_id, &comp_info.id);
            if let Some(cached) = self.state_cache.get_component(&composite_key) {
                cached
            } else {
                // Would need to convert ComponentInfo to ProtocolComponent
                // For now, require component to be in cache
                return Err(BridgeError::component_not_found(&comp_info.id));
            }
        } else {
            // Get from cache using swap_info.component_id
            let composite_key = make_composite_key(chain_id, &swap_info.component_id);
            self.state_cache
                .get_component(&composite_key)
                .ok_or_else(|| BridgeError::component_not_found(&swap_info.component_id))?
        };

        // Parse tokens
        let token_in = parse_address(&swap_info.token_in).map_err(|e| {
            BridgeError::invalid_argument(format!("Invalid token_in in swap {}: {}", idx, e))
        })?;
        let token_out = parse_address(&swap_info.token_out).map_err(|e| {
            BridgeError::invalid_argument(format!("Invalid token_out in swap {}: {}", idx, e))
        })?;

        // Build swap (default 100% split unless we add split field to SwapInfo)
        let swap = SwapBuilder::new(component, token_in, token_out)
            .split(1.0)
            .build();

        Ok(swap)
    }
}

/// Parse a hex address string to Bytes.
fn parse_address(addr: &str) -> BridgeResult<Bytes> {
    let normalized = addr.strip_prefix("0x").unwrap_or(addr);
    Bytes::from_str(normalized)
        .map_err(|_| BridgeError::invalid_argument(format!("Invalid address: {}", addr)))
}
