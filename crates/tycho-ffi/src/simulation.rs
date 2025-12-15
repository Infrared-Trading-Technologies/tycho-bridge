//! FFI simulation functions.
//!
//! Provides C-compatible exports for all simulation operations:
//! - `tycho_spot_price` - Get spot price for a token pair
//! - `tycho_get_amount_out` - Simulate a swap
//! - `tycho_get_fee` - Get pool fee
//! - `tycho_get_limits` - Get trading limits
//! - `tycho_swap_to_price` - Calculate swap to reach target price
//! - `tycho_query_supply` - Query supply curve
//! - `tycho_batch_get_amount_out` - Batch swap simulations
//! - `tycho_simulate_route` - Multi-hop route simulation
//! - `tycho_get_tokens` - Get all tokens
//! - `tycho_get_components` - Get protocol components
//! - `tycho_get_status` - Get indexer status

use crate::bridge::get_context;
use crate::error::{clear_last_error, set_last_bridge_error, set_last_error, FfiErrorCode};
use crate::types::{
    cstr_to_biguint, cstr_to_string, FfiBatchGetAmountOutRequest, FfiBridge,
    FfiGetAmountOutRequest, FfiGetComponentsRequest, FfiGetFeeRequest, FfiGetLimitsRequest,
    FfiGetStatusRequest, FfiGetTokensRequest, FfiQuerySupplyRequest, FfiResult,
    FfiSimulateRouteRequest, FfiSpotPriceRequest, FfiSwapToPriceRequest,
};
use tycho_core::types::request as core_req;

// ============================================================================
// Simulation Functions
// ============================================================================

/// Get spot price for a token pair in a pool.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - Spot price request parameters
///
/// # Returns
/// FfiResult containing JSON response: `{"numerator": "...", "denominator": "..."}`
///
/// # Safety
/// - `handle` must be a valid bridge handle from `tycho_bridge_init`
/// - All string pointers in `request` must be valid null-terminated UTF-8
#[no_mangle]
pub unsafe extern "C" fn tycho_spot_price(
    handle: *const FfiBridge,
    request: *const FfiSpotPriceRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = match (*request).to_core() {
        Some(r) => r,
        None => {
            set_last_error(
                FfiErrorCode::InvalidArgument,
                "Failed to parse request fields",
            );
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
        }
    };

    match ctx.bridge().spot_price(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Simulate a swap to get output amount.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - GetAmountOut request parameters
///
/// # Returns
/// FfiResult containing JSON response with amount_out, gas_cost, state_id, execution_price
#[no_mangle]
pub unsafe extern "C" fn tycho_get_amount_out(
    handle: *const FfiBridge,
    request: *const FfiGetAmountOutRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = match (*request).to_core() {
        Some(r) => r,
        None => {
            set_last_error(
                FfiErrorCode::InvalidArgument,
                "Failed to parse request fields",
            );
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
        }
    };

    match ctx.bridge().get_amount_out(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Get the fee for a pool/token.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - GetFee request parameters
///
/// # Returns
/// FfiResult containing JSON response with fee_numerator, fee_denominator
#[no_mangle]
pub unsafe extern "C" fn tycho_get_fee(
    handle: *const FfiBridge,
    request: *const FfiGetFeeRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = match (*request).to_core() {
        Some(r) => r,
        None => {
            set_last_error(
                FfiErrorCode::InvalidArgument,
                "Failed to parse request fields",
            );
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
        }
    };

    match ctx.bridge().get_fee(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Get trading limits for a pool.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - GetLimits request parameters
///
/// # Returns
/// FfiResult containing JSON response with min_amount_in, max_amount_in
#[no_mangle]
pub unsafe extern "C" fn tycho_get_limits(
    handle: *const FfiBridge,
    request: *const FfiGetLimitsRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = match (*request).to_core() {
        Some(r) => r,
        None => {
            set_last_error(
                FfiErrorCode::InvalidArgument,
                "Failed to parse request fields",
            );
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
        }
    };

    match ctx.bridge().get_limits(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Calculate swap needed to reach a target price.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - SwapToPrice request parameters
///
/// # Returns
/// FfiResult containing JSON response with trade info
#[no_mangle]
pub unsafe extern "C" fn tycho_swap_to_price(
    handle: *const FfiBridge,
    request: *const FfiSwapToPriceRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = match (*request).to_core() {
        Some(r) => r,
        None => {
            set_last_error(
                FfiErrorCode::InvalidArgument,
                "Failed to parse request fields",
            );
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
        }
    };

    match ctx.bridge().swap_to_price(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Query the supply curve for a pool.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - QuerySupply request parameters
///
/// # Returns
/// FfiResult containing JSON response with trades array
#[no_mangle]
pub unsafe extern "C" fn tycho_query_supply(
    handle: *const FfiBridge,
    request: *const FfiQuerySupplyRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = match (*request).to_core() {
        Some(r) => r,
        None => {
            set_last_error(
                FfiErrorCode::InvalidArgument,
                "Failed to parse request fields",
            );
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
        }
    };

    match ctx.bridge().query_supply(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Batch simulate multiple swaps in parallel.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - Batch request with array of GetAmountOut requests
///
/// # Returns
/// FfiResult containing JSON response with results array
#[no_mangle]
pub unsafe extern "C" fn tycho_batch_get_amount_out(
    handle: *const FfiBridge,
    request: *const FfiBatchGetAmountOutRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let batch_req = &*request;
    if batch_req.requests.is_null() || batch_req.count == 0 {
        set_last_error(FfiErrorCode::InvalidArgument, "Empty batch request");
        return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Empty batch");
    }

    // Convert all requests
    let ffi_requests = std::slice::from_raw_parts(batch_req.requests, batch_req.count);
    let mut core_requests = Vec::with_capacity(batch_req.count);

    for ffi_req in ffi_requests {
        match ffi_req.to_core() {
            Some(r) => core_requests.push(r),
            None => {
                set_last_error(
                    FfiErrorCode::InvalidArgument,
                    "Failed to parse batch request",
                );
                return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid request");
            }
        }
    }

    let req = core_req::BatchGetAmountOutRequest {
        requests: core_requests,
    };

    match ctx.bridge().batch_get_amount_out(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Simulate a multi-hop route.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - Route simulation request
///
/// # Returns
/// FfiResult containing JSON response with final_amount_out, total_gas_cost, hop_results
#[no_mangle]
pub unsafe extern "C" fn tycho_simulate_route(
    handle: *const FfiBridge,
    request: *const FfiSimulateRouteRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let route_req = &*request;
    if route_req.hops.is_null() || route_req.hop_count == 0 {
        set_last_error(FfiErrorCode::InvalidArgument, "Empty route");
        return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Empty route");
    }

    // Convert hops
    let ffi_hops = std::slice::from_raw_parts(route_req.hops, route_req.hop_count);
    let mut hops = Vec::with_capacity(route_req.hop_count);

    for hop in ffi_hops {
        let component_id = match cstr_to_string(hop.component_id) {
            Some(s) => s,
            None => {
                set_last_error(FfiErrorCode::InvalidArgument, "Invalid hop component_id");
                return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid hop");
            }
        };
        let token_in = match cstr_to_string(hop.token_in) {
            Some(s) => s,
            None => {
                set_last_error(FfiErrorCode::InvalidArgument, "Invalid hop token_in");
                return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid hop");
            }
        };
        let token_out = match cstr_to_string(hop.token_out) {
            Some(s) => s,
            None => {
                set_last_error(FfiErrorCode::InvalidArgument, "Invalid hop token_out");
                return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid hop");
            }
        };

        hops.push(core_req::RouteHop {
            component_id,
            token_in,
            token_out,
        });
    }

    let amount_in = match cstr_to_biguint(route_req.amount_in) {
        Some(a) => a,
        None => {
            set_last_error(FfiErrorCode::InvalidArgument, "Invalid amount_in");
            return FfiResult::error(FfiErrorCode::InvalidArgument as i32, "Invalid amount");
        }
    };

    let req = core_req::SimulateRouteRequest {
        chain_id: route_req.chain_id,
        hops,
        amount_in,
        state_overrides: std::collections::HashMap::new(),
    };

    match ctx.bridge().simulate_route(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

// ============================================================================
// Indexing Functions
// ============================================================================

/// Get all available tokens.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - GetTokens request parameters
///
/// # Returns
/// FfiResult containing JSON response with tokens array
#[no_mangle]
pub unsafe extern "C" fn tycho_get_tokens(
    handle: *const FfiBridge,
    request: *const FfiGetTokensRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = (*request).to_core();

    match ctx.bridge().get_tokens(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Get protocol components.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - GetComponents request parameters
///
/// # Returns
/// FfiResult containing JSON response with components array
#[no_mangle]
pub unsafe extern "C" fn tycho_get_components(
    handle: *const FfiBridge,
    request: *const FfiGetComponentsRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = (*request).to_core();

    match ctx.bridge().get_components(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

// ============================================================================
// Status Functions
// ============================================================================

/// Get indexer sync status.
///
/// # Arguments
/// * `handle` - Bridge handle
/// * `request` - GetStatus request parameters
///
/// # Returns
/// FfiResult containing JSON response with sync_status, latest_block, etc.
#[no_mangle]
pub unsafe extern "C" fn tycho_get_status(
    handle: *const FfiBridge,
    request: *const FfiGetStatusRequest,
) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    if request.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "request is null");
        return FfiResult::null_error();
    }

    let req = (*request).to_core();

    match ctx.bridge().get_status(req) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}

/// Perform a health check.
///
/// # Arguments
/// * `handle` - Bridge handle
///
/// # Returns
/// FfiResult containing JSON response with healthy status
#[no_mangle]
pub unsafe extern "C" fn tycho_health_check(handle: *const FfiBridge) -> FfiResult {
    clear_last_error();

    let ctx = match get_context(handle) {
        Some(c) => c,
        None => return FfiResult::not_initialized(),
    };

    match ctx.bridge().health_check() {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(json) => FfiResult::success(json),
            Err(e) => {
                set_last_error(FfiErrorCode::JsonError, e.to_string());
                FfiResult::error(FfiErrorCode::JsonError as i32, "JSON serialization failed")
            }
        },
        Err(e) => {
            set_last_bridge_error(&e);
            FfiResult::error(e.code(), &e.to_string())
        }
    }
}
