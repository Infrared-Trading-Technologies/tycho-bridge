//! Simulation handlers - ProtocolSim trait methods.
//!
//! This module implements:
//! - `spot_price` - Marginal price query
//! - `get_amount_out` - Swap simulation (primary)
//! - `get_fee` - Protocol fee query
//! - `get_limits` - Trading limits query
//! - `swap_to_price` - Price-targeting trade
//! - `query_supply` - Supply curve query
//! - `batch_get_amount_out` - Parallel simulations
//! - `simulate_route` - Multi-hop simulation

use std::time::Instant;

use num_bigint::BigUint;
use num_traits::Zero;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::bridge::BridgeService;
use crate::domain::Price as TychoPrice;
use crate::error::{BridgeError, BridgeResult};
use crate::helpers::{calculate_execution_price, find_token_in_component, normalize_address};
use crate::state::make_composite_key;
use crate::types::request::{
    BatchGetAmountOutRequest, GetAmountOutRequest, GetFeeRequest, GetLimitsRequest,
    QuerySupplyRequest, SimulateRouteRequest, SpotPriceRequest, SwapToPriceRequest,
};
use crate::types::response::{
    BatchGetAmountOutResponse, ExecutionPrice, GetAmountOutResponse, GetFeeResponse,
    GetLimitsResponse, HopResult, QuerySupplyResponse, SimulateRouteResponse, SpotPriceResponse,
    SwapToPriceResponse, TradeInfo,
};

impl BridgeService {
    /// Get the spot price for a token pair in a pool.
    ///
    /// Returns the marginal price as base_token/quote_token.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn spot_price(&self, req: SpotPriceRequest) -> BridgeResult<SpotPriceResponse> {
        self.validate_chain(req.chain_id)?;

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        // Get state from cache
        let state = self
            .state_cache
            .get_state(&composite_key)
            .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?;

        // Get component to find tokens
        let component = self
            .state_cache
            .get_component(&composite_key)
            .ok_or_else(|| BridgeError::internal("Component metadata not found"))?;

        // Normalize addresses and find tokens
        let base_addr = normalize_address(&req.base_token)?;
        let quote_addr = normalize_address(&req.quote_token)?;

        let base_token = find_token_in_component(&component.tokens, &base_addr)
            .ok_or_else(|| BridgeError::token_not_found(&req.base_token))?;

        let quote_token = find_token_in_component(&component.tokens, &quote_addr)
            .ok_or_else(|| BridgeError::token_not_found(&req.quote_token))?;

        // Get spot price from state
        let state_guard = state.read().map_err(|e| {
            error!(error = %e, "Failed to acquire read lock");
            BridgeError::internal("Failed to lock state")
        })?;

        let price = state_guard
            .spot_price(&base_token, &quote_token)
            .map_err(|e| {
                warn!(error = %e, "SpotPrice failed");
                BridgeError::SimulationError {
                    message: e.to_string(),
                }
            })?;

        // Convert float to rational (approximate using fixed precision)
        let price_info = crate::types::from_f64(price);
        let numerator = price_info.numerator.to_biguint().unwrap_or_default();
        let denominator = price_info
            .denominator
            .to_biguint()
            .unwrap_or(BigUint::from(1u32));

        debug!(component_id = %req.component_id, price = %price, "SpotPrice completed");

        Ok(SpotPriceResponse {
            numerator,
            denominator,
        })
    }

    /// Simulate a swap and get the output amount.
    ///
    /// This is the primary simulation method. Returns the output amount,
    /// gas cost, and optionally saves the new state for chaining.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn get_amount_out(&self, req: GetAmountOutRequest) -> BridgeResult<GetAmountOutResponse> {
        self.validate_chain(req.chain_id)?;
        let start_time = Instant::now();

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        // Get state from cache (or use override if state_id is provided)
        let state = if let Some(state_id) = req.state_id {
            self.transient_state_cache.get(&state_id).ok_or_else(|| {
                BridgeError::StateUnavailable {
                    message: format!("Transient state {} (may have expired)", state_id),
                }
            })?
        } else {
            self.state_cache
                .get_state(&composite_key)
                .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?
        };

        // Get component to find tokens
        let component = self
            .state_cache
            .get_component(&composite_key)
            .ok_or_else(|| BridgeError::internal("Component metadata not found"))?;

        // Validate amount
        if req.amount_in.is_zero() {
            return Err(BridgeError::invalid_argument("amount_in must be > 0"));
        }

        // Normalize addresses and find tokens
        let token_in_addr = normalize_address(&req.token_in)?;
        let token_out_addr = normalize_address(&req.token_out)?;

        let token_in = find_token_in_component(&component.tokens, &token_in_addr)
            .ok_or_else(|| BridgeError::token_not_found(&req.token_in))?;

        let token_out = find_token_in_component(&component.tokens, &token_out_addr)
            .ok_or_else(|| BridgeError::token_not_found(&req.token_out))?;

        // Execute simulation
        let state_guard = state.read().map_err(|e| {
            error!(error = %e, "Failed to acquire read lock");
            BridgeError::internal("Failed to lock state")
        })?;

        let sim_result = state_guard
            .get_amount_out(req.amount_in.clone(), &token_in, &token_out)
            .map_err(|e| {
                warn!(error = %e, "GetAmountOut simulation failed");
                BridgeError::SimulationError {
                    message: e.to_string(),
                }
            })?;

        // Drop the guard before storing new state
        drop(state_guard);

        // Calculate execution price
        let effective_price = calculate_execution_price(
            &req.amount_in,
            &sim_result.amount,
            token_in.decimals,
            token_out.decimals,
        );

        // Store the new state in transient cache if requested
        let state_id = if req.save_state {
            Some(self.transient_state_cache.store(
                sim_result.new_state,
                req.chain_id,
                &req.component_id,
            ))
        } else {
            None
        };

        let elapsed = start_time.elapsed();

        info!(
            component_id = %req.component_id,
            amount_in = %req.amount_in,
            amount_out = %sim_result.amount,
            gas = %sim_result.gas,
            state_saved = state_id.is_some(),
            duration_us = elapsed.as_micros(),
            "GetAmountOut completed"
        );

        // Convert effective_price to rational
        let price_info = crate::types::from_f64(effective_price);
        let numerator = price_info.numerator.to_biguint().unwrap_or_default();
        let denominator = price_info
            .denominator
            .to_biguint()
            .unwrap_or(BigUint::from(1u32));

        Ok(GetAmountOutResponse {
            amount_out: sim_result.amount,
            gas_cost: sim_result.gas,
            state_id,
            execution_price: Some(ExecutionPrice {
                numerator,
                denominator,
            }),
        })
    }

    /// Get the protocol fee for a token pair.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn get_fee(&self, req: GetFeeRequest) -> BridgeResult<GetFeeResponse> {
        self.validate_chain(req.chain_id)?;

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        let state = self
            .state_cache
            .get_state(&composite_key)
            .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?;

        let state_guard = state
            .read()
            .map_err(|_| BridgeError::internal("Lock error"))?;
        let fee = state_guard.fee();

        // Convert fee (f64 fraction) to rational
        let fee_bps = (fee * 10000.0).round() as u64;

        debug!(component_id = %req.component_id, fee = %fee, fee_bps = %fee_bps, "GetFee completed");

        Ok(GetFeeResponse {
            fee_numerator: BigUint::from(fee_bps),
            fee_denominator: BigUint::from(10000u32),
        })
    }

    /// Get trading limits for a token pair.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn get_limits(&self, req: GetLimitsRequest) -> BridgeResult<GetLimitsResponse> {
        self.validate_chain(req.chain_id)?;

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        let state = self
            .state_cache
            .get_state(&composite_key)
            .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?;

        // Parse token addresses
        let token_in_bytes = crate::helpers::parse_address(&req.token_in)?;
        let token_out_bytes = crate::helpers::parse_address(&req.token_out)?;

        let state_guard = state.read().map_err(|e| {
            error!(error = %e, "Failed to acquire read lock on state");
            BridgeError::internal("Failed to lock state")
        })?;

        let (max_sell_amount, _max_buy_amount) = state_guard
            .get_limits(token_in_bytes, token_out_bytes)
            .map_err(|e| {
                warn!(error = %e, "GetLimits simulation failed");
                BridgeError::SimulationError {
                    message: e.to_string(),
                }
            })?;

        info!(
            component_id = %req.component_id,
            max_in = %max_sell_amount,
            "GetLimits completed"
        );

        Ok(GetLimitsResponse {
            min_amount_in: BigUint::from(1u32),
            max_amount_in: max_sell_amount,
        })
    }

    /// Calculate the trade needed to reach a target price.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn swap_to_price(&self, req: SwapToPriceRequest) -> BridgeResult<SwapToPriceResponse> {
        self.validate_chain(req.chain_id)?;

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        let state = self
            .state_cache
            .get_state(&composite_key)
            .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?;

        // Parse token addresses
        let token_in_bytes = crate::helpers::parse_address(&req.token_in)?;
        let token_out_bytes = crate::helpers::parse_address(&req.token_out)?;

        // Build Tycho Price struct
        let tycho_price = TychoPrice {
            numerator: req.target_price_numerator.clone(),
            denominator: req.target_price_denominator.clone(),
        };

        let state_guard = state.read().map_err(|e| {
            error!(error = %e, "Failed to acquire read lock on state");
            BridgeError::internal("Failed to lock state")
        })?;

        match state_guard.swap_to_price(&token_in_bytes, &token_out_bytes, tycho_price) {
            Ok(trade) => {
                let is_reachable = !trade.amount_in.is_zero() || !trade.amount_out.is_zero();

                info!(
                    component_id = %req.component_id,
                    amount_in = %trade.amount_in,
                    amount_out = %trade.amount_out,
                    reached = is_reachable,
                    "SwapToPrice completed"
                );

                Ok(SwapToPriceResponse {
                    trade: Some(TradeInfo {
                        amount_in: trade.amount_in,
                        amount_out: trade.amount_out,
                        price_numerator: req.target_price_numerator,
                        price_denominator: req.target_price_denominator,
                    }),
                    reached_target: is_reachable,
                })
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                let is_unreachable = error_msg.contains("price")
                    && (error_msg.contains("not reachable")
                        || error_msg.contains("unreachable")
                        || error_msg.contains("cannot reach"));

                if is_unreachable {
                    info!(component_id = %req.component_id, "SwapToPrice - price not reachable");
                    Ok(SwapToPriceResponse {
                        trade: None,
                        reached_target: false,
                    })
                } else {
                    warn!(error = %e, "SwapToPrice simulation failed");
                    Err(BridgeError::SimulationError {
                        message: e.to_string(),
                    })
                }
            }
        }
    }

    /// Query the supply curve.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn query_supply(&self, req: QuerySupplyRequest) -> BridgeResult<QuerySupplyResponse> {
        self.validate_chain(req.chain_id)?;

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        let state = self
            .state_cache
            .get_state(&composite_key)
            .ok_or_else(|| BridgeError::component_not_found(&req.component_id))?;

        // Parse token addresses
        let token_in_bytes = crate::helpers::parse_address(&req.token_in)?;
        let token_out_bytes = crate::helpers::parse_address(&req.token_out)?;

        // Build min_price
        let min_price = TychoPrice {
            numerator: req.min_price_numerator.clone(),
            denominator: req.min_price_denominator.clone(),
        };

        let state_guard = state.read().map_err(|e| {
            error!(error = %e, "Failed to acquire read lock on state");
            BridgeError::internal("Failed to lock state")
        })?;

        let trade = state_guard
            .query_supply(&token_in_bytes, &token_out_bytes, min_price)
            .map_err(|e| {
                warn!(error = %e, "QuerySupply simulation failed");
                BridgeError::SimulationError {
                    message: e.to_string(),
                }
            })?;

        info!(
            component_id = %req.component_id,
            amount_in = %trade.amount_in,
            amount_out = %trade.amount_out,
            "QuerySupply completed"
        );

        Ok(QuerySupplyResponse {
            trades: vec![TradeInfo {
                amount_in: trade.amount_in,
                amount_out: trade.amount_out,
                price_numerator: req.min_price_numerator,
                price_denominator: req.min_price_denominator,
            }],
        })
    }

    /// Batch get amounts out for multiple requests.
    #[instrument(skip(self), fields(batch_size = req.requests.len()))]
    pub fn batch_get_amount_out(
        &self,
        req: BatchGetAmountOutRequest,
    ) -> BridgeResult<BatchGetAmountOutResponse> {
        let start_time = Instant::now();
        let batch_size = req.requests.len();

        if batch_size == 0 {
            return Ok(BatchGetAmountOutResponse { results: vec![] });
        }

        let results: Vec<_> = req
            .requests
            .into_iter()
            .map(|single_req| self.get_amount_out(single_req).ok())
            .collect();

        let success_count = results.iter().filter(|r| r.is_some()).count();
        let elapsed = start_time.elapsed();

        info!(
            batch_size,
            success_count,
            error_count = batch_size - success_count,
            duration_us = elapsed.as_micros(),
            "BatchGetAmountOut completed"
        );

        Ok(BatchGetAmountOutResponse { results })
    }

    /// Simulate a multi-hop route with state chaining.
    #[instrument(skip(self), fields(hop_count = req.hops.len()))]
    pub fn simulate_route(&self, req: SimulateRouteRequest) -> BridgeResult<SimulateRouteResponse> {
        let start_time = Instant::now();
        let hop_count = req.hops.len();

        if hop_count == 0 {
            return Err(BridgeError::invalid_argument(
                "At least one hop is required",
            ));
        }

        self.validate_chain(req.chain_id)?;

        if req.amount_in.is_zero() {
            return Err(BridgeError::invalid_argument("amount_in must be > 0"));
        }

        let mut current_amount = req.amount_in.clone();
        let mut hop_results: Vec<HopResult> = Vec::with_capacity(hop_count);
        let mut total_gas = BigUint::from(0u64);

        // Track modified states for pools we've simulated
        let mut pool_states: std::collections::HashMap<String, crate::state::SharedState> =
            std::collections::HashMap::new();

        // Load any pre-existing state overrides
        for (component_id, state_id) in req.state_overrides.iter() {
            if let Some(cached_state) = self.transient_state_cache.get(state_id) {
                pool_states.insert(component_id.clone(), cached_state);
            }
        }

        // Process each hop
        for (hop_index, hop) in req.hops.iter().enumerate() {
            let composite_key = make_composite_key(req.chain_id, &hop.component_id);

            let state = if let Some(chained_state) = pool_states.get(&hop.component_id) {
                trace!(hop_index, component_id = %hop.component_id, "Using chained state");
                chained_state.clone()
            } else {
                self.state_cache.get_state(&composite_key).ok_or_else(|| {
                    BridgeError::component_not_found(format!(
                        "{} (hop {})",
                        hop.component_id, hop_index
                    ))
                })?
            };

            let component = self
                .state_cache
                .get_component(&composite_key)
                .ok_or_else(|| {
                    BridgeError::internal(format!(
                        "Component metadata not found (hop {})",
                        hop_index
                    ))
                })?;

            let token_in_addr = normalize_address(&hop.token_in)?;
            let token_out_addr = normalize_address(&hop.token_out)?;

            let token_in =
                find_token_in_component(&component.tokens, &token_in_addr).ok_or_else(|| {
                    BridgeError::token_not_found(format!("{} (hop {})", hop.token_in, hop_index))
                })?;

            let token_out = find_token_in_component(&component.tokens, &token_out_addr)
                .ok_or_else(|| {
                    BridgeError::token_not_found(format!("{} (hop {})", hop.token_out, hop_index))
                })?;

            let state_guard = state.read().map_err(|e| {
                error!(error = %e, hop_index, "Failed to acquire read lock");
                BridgeError::internal(format!("Failed to lock state for hop {}", hop_index))
            })?;

            let sim_result = state_guard
                .get_amount_out(current_amount.clone(), &token_in, &token_out)
                .map_err(|e| {
                    warn!(error = %e, hop_index, "Hop simulation failed");
                    BridgeError::SimulationError {
                        message: format!("Simulation failed at hop {}: {}", hop_index, e),
                    }
                })?;

            drop(state_guard);

            hop_results.push(HopResult {
                component_id: hop.component_id.clone(),
                amount_out: sim_result.amount.clone(),
                gas_cost: sim_result.gas.clone(),
            });

            total_gas += &sim_result.gas;

            let new_state: crate::state::SharedState =
                std::sync::Arc::new(std::sync::RwLock::new(sim_result.new_state));
            pool_states.insert(hop.component_id.clone(), new_state);

            current_amount = sim_result.amount;
        }

        let elapsed = start_time.elapsed();

        info!(
            hop_count,
            amount_in = %req.amount_in,
            amount_out = %current_amount,
            total_gas = %total_gas,
            duration_us = elapsed.as_micros(),
            "SimulateRoute completed"
        );

        Ok(SimulateRouteResponse {
            final_amount_out: current_amount,
            total_gas_cost: total_gas,
            hop_results,
        })
    }
}
