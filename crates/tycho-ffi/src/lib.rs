//! Tycho FFI - C-compatible foreign function interface for tycho-core.
//!
//! This crate exposes tycho-core functionality via C ABI for use by
//! other languages (Go, Python, Node.js, etc.)
//!
//! # Architecture
//!
//! The FFI layer provides:
//! - Bridge initialization and lifecycle management
//! - Simulation functions (spot_price, get_amount_out, etc.)
//! - Indexing functions (get_tokens, get_components)
//! - Status functions (get_status, health_check)
//! - Memory management functions
//! - Thread-local error handling
//!
//! # Memory Management
//!
//! All memory allocated by Rust functions must be freed using the
//! corresponding `tycho_free_*` functions. Each FFI function returns
//! an `FfiResult` that contains either:
//! - JSON data on success (must be freed with `tycho_free_result`)
//! - Error information on failure
//!
//! # Error Handling
//!
//! Errors are reported in two ways:
//! 1. Via the `FfiResult` return value (error_code, error_message)
//! 2. Via thread-local last error storage (for functions that return handles)
//!
//! Use `tycho_last_error_code()` and `tycho_last_error_message()` to get
//! detailed error information.
//!
//! # Thread Safety
//!
//! The bridge handle can be safely shared between threads. Each handle
//! contains its own async runtime, and all operations are thread-safe.
//!
//! # Example Usage (C)
//!
//! ```c
//! #include "tycho_bridge.h"
//!
//! // Initialize bridge
//! const char* config = "{\"tycho_url\": \"...\", \"chain_ids\": \"1\"}";
//! FfiBridge* bridge = tycho_bridge_init(config);
//! if (bridge == NULL) {
//!     printf("Error: %s\n", tycho_last_error_message());
//!     return 1;
//! }
//!
//! // Start sync
//! tycho_bridge_start(bridge);
//!
//! // Make a query
//! FfiSpotPriceRequest req = {
//!     .chain_id = 1,
//!     .component_id = "pool_id",
//!     .base_token = "0x...",
//!     .quote_token = "0x...",
//! };
//! FfiResult result = tycho_spot_price(bridge, &req);
//!
//! if (result.success) {
//!     // Parse JSON from result.data (result.data_len bytes)
//! } else {
//!     printf("Error %d: %s\n", result.error_code, result.error_message);
//! }
//!
//! tycho_free_result(result);
//! tycho_bridge_free(bridge);
//! ```

#![allow(clippy::missing_safety_doc)]

pub mod bridge;
pub mod error;
pub mod memory;
mod simulation;
pub mod types;

// Re-export all public FFI functions for cbindgen
pub use bridge::{
    tycho_bridge_component_count, tycho_bridge_free, tycho_bridge_init, tycho_bridge_is_healthy,
    tycho_bridge_start, tycho_bridge_stop, tycho_bridge_uptime, tycho_version,
};
pub use error::{
    tycho_clear_last_error, tycho_error_code_description, tycho_has_error, tycho_last_error_code,
    tycho_last_error_length, tycho_last_error_message, FfiErrorCode,
};
pub use memory::{
    tycho_alloc, tycho_buffer_data, tycho_buffer_len, tycho_free_buffer, tycho_free_result,
    tycho_free_string, tycho_result_to_json_string, tycho_strdup, tycho_string_len,
    tycho_string_ptr,
};
pub use simulation::{
    tycho_batch_get_amount_out, tycho_get_amount_out, tycho_get_components, tycho_get_fee,
    tycho_get_limits, tycho_get_status, tycho_get_tokens, tycho_health_check, tycho_query_supply,
    tycho_simulate_route, tycho_spot_price, tycho_swap_to_price,
};
pub use types::{
    FfiBatchGetAmountOutRequest, FfiBridge, FfiBuffer, FfiGetAmountOutRequest,
    FfiGetComponentsRequest, FfiGetFeeRequest, FfiGetLimitsRequest, FfiGetStatusRequest,
    FfiGetTokensRequest, FfiQuerySupplyRequest, FfiResult, FfiRouteHop, FfiSimulateRouteRequest,
    FfiSpotPriceRequest, FfiString, FfiSwapToPriceRequest,
};
