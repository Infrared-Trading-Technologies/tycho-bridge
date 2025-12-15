//! FFI bridge for C/Go interop.
//!
//! This module provides the main bridge instance management functions:
//! - `tycho_bridge_init` - Initialize a bridge from JSON config
//! - `tycho_bridge_free` - Free a bridge instance
//! - `tycho_bridge_start` - Start background sync
//! - `tycho_bridge_stop` - Stop background sync

use std::ffi::{c_char, CStr};
use std::sync::Arc;

use tycho_core::{BridgeService, Config};

use crate::error::{clear_last_error, set_last_error, FfiErrorCode};
use crate::types::{FfiBridge, FfiResult};

// ============================================================================
// Bridge Context (internal)
// ============================================================================

/// Internal context holding the async runtime and bridge service.
pub(crate) struct BridgeContext {
    /// Tokio runtime for async operations
    pub runtime: tokio::runtime::Runtime,
    /// The bridge service instance
    pub bridge: Arc<BridgeService>,
}

impl BridgeContext {
    /// Get a reference to the bridge service.
    pub fn bridge(&self) -> &BridgeService {
        &self.bridge
    }

    /// Run an async function on the runtime.
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

// ============================================================================
// Bridge Initialization
// ============================================================================

/// Initialize a new bridge instance from JSON configuration.
///
/// # Arguments
/// * `config_json` - JSON string containing bridge configuration
///
/// # Returns
/// An opaque handle to the bridge, or null on failure.
/// Call `tycho_last_error_code()` and `tycho_last_error_message()` for error details.
///
/// # Safety
/// - `config_json` must be a valid, null-terminated UTF-8 string.
/// - The returned handle must be freed with `tycho_bridge_free`.
///
/// # Example JSON Config
/// ```json
/// {
///   "tycho_url": "tycho-beta.propellerheads.xyz",
///   "tycho_api_key": "your-api-key",
///   "chain_ids": "1,8453"
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_init(config_json: *const c_char) -> *mut FfiBridge {
    clear_last_error();

    // Validate input
    if config_json.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "config_json is null");
        return std::ptr::null_mut();
    }

    // Parse config string
    let config_str = match CStr::from_ptr(config_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(FfiErrorCode::InvalidUtf8, format!("Invalid UTF-8 in config: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Parse JSON config
    let config: Config = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            set_last_error(FfiErrorCode::ConfigError, format!("Invalid config JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Create tokio runtime
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            set_last_error(FfiErrorCode::InternalError, format!("Failed to create runtime: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Initialize bridge service
    let bridge = match runtime.block_on(BridgeService::from_config(&config)) {
        Ok(b) => Arc::new(b),
        Err(e) => {
            set_last_error(FfiErrorCode::from(&e), e.to_string());
            return std::ptr::null_mut();
        }
    };

    // Box the context and convert to raw pointer
    let ctx = Box::new(BridgeContext { runtime, bridge });
    Box::into_raw(ctx) as *mut FfiBridge
}

/// Free a bridge instance.
///
/// # Arguments
/// * `handle` - The bridge handle to free (may be null, which is a no-op)
///
/// # Safety
/// - The handle must have been returned by `tycho_bridge_init`.
/// - Do not use the handle after calling this function.
/// - Double-free will cause undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_free(handle: *mut FfiBridge) {
    if handle.is_null() {
        return;
    }

    // Reconstruct the Box and let it drop
    let _ = Box::from_raw(handle as *mut BridgeContext);
}

/// Start the background synchronization task.
///
/// This starts syncing component state from the Tycho indexer.
/// The sync runs in the background and updates the state cache.
///
/// # Arguments
/// * `handle` - The bridge handle
///
/// # Returns
/// FfiResult with success or error status.
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_start(handle: *mut FfiBridge) -> FfiResult {
    clear_last_error();

    if handle.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "handle is null");
        return FfiResult::null_error();
    }

    let ctx = &*(handle as *const BridgeContext);

    // Start the background sync by connecting
    let result = ctx.block_on(async {
        ctx.bridge.tycho_manager().connect().await
    });

    match result {
        Ok(_handle) => FfiResult::success(b"{\"status\": \"started\"}".to_vec()),
        Err(e) => {
            set_last_error(FfiErrorCode::ConnectionError, e.to_string());
            FfiResult::error(FfiErrorCode::ConnectionError as i32, &e.to_string())
        }
    }
}

/// Stop the background synchronization task.
///
/// Note: The current TychoManager doesn't expose a stop method.
/// This is a placeholder that returns success.
///
/// # Arguments
/// * `handle` - The bridge handle
///
/// # Returns
/// FfiResult with success status.
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_stop(handle: *mut FfiBridge) -> FfiResult {
    clear_last_error();

    if handle.is_null() {
        set_last_error(FfiErrorCode::NullPointer, "handle is null");
        return FfiResult::null_error();
    }

    // Note: TychoManager doesn't currently expose a stop method.
    // The connection is stopped when the bridge is freed.
    // This is a no-op for now.
    FfiResult::success(b"{\"status\": \"stopped\"}".to_vec())
}

// ============================================================================
// Bridge Status Functions
// ============================================================================

/// Get the bridge version string.
///
/// # Returns
/// A static string containing the version.
#[no_mangle]
pub extern "C" fn tycho_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

/// Check if the bridge is healthy.
///
/// # Arguments
/// * `handle` - The bridge handle
///
/// # Returns
/// True if the bridge is healthy, false otherwise.
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_is_healthy(handle: *const FfiBridge) -> bool {
    if handle.is_null() {
        return false;
    }

    let ctx = &*(handle as *const BridgeContext);

    // Check basic health - can we access the state cache?
    ctx.bridge.state_cache().component_count() > 0
        || ctx.bridge.uptime_seconds() > 0
}

/// Get the number of components currently cached.
///
/// # Arguments
/// * `handle` - The bridge handle
///
/// # Returns
/// Number of components, or 0 if handle is null.
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_component_count(handle: *const FfiBridge) -> usize {
    if handle.is_null() {
        return 0;
    }

    let ctx = &*(handle as *const BridgeContext);
    ctx.bridge.state_cache().component_count()
}

/// Get the bridge uptime in seconds.
///
/// # Arguments
/// * `handle` - The bridge handle
///
/// # Returns
/// Uptime in seconds, or 0 if handle is null.
#[no_mangle]
pub unsafe extern "C" fn tycho_bridge_uptime(handle: *const FfiBridge) -> u64 {
    if handle.is_null() {
        return 0;
    }

    let ctx = &*(handle as *const BridgeContext);
    ctx.bridge.uptime_seconds()
}

// ============================================================================
// Internal Helper to Get Context
// ============================================================================

/// Get the bridge context from a handle, with null check.
/// Returns None and sets last error if handle is null.
pub(crate) unsafe fn get_context(handle: *const FfiBridge) -> Option<&'static BridgeContext> {
    if handle.is_null() {
        set_last_error(FfiErrorCode::NotInitialized, "Bridge handle is null");
        return None;
    }
    Some(&*(handle as *const BridgeContext))
}
