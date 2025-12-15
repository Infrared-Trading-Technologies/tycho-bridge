//! FFI type definitions.
//!
//! Provides C-compatible type definitions for FFI.
//! All types use #[repr(C)] for stable ABI.

use std::ffi::c_char;

// ============================================================================
// Core FFI Types
// ============================================================================

/// Opaque handle to the bridge instance.
/// The actual structure is hidden from C callers.
#[repr(C)]
pub struct FfiBridge {
    _private: [u8; 0],
}

/// A byte buffer for passing binary data across FFI.
#[repr(C)]
pub struct FfiBuffer {
    /// Pointer to data (may be null if len is 0)
    pub data: *mut u8,
    /// Length of data in bytes
    pub len: usize,
    /// Capacity of allocated buffer
    pub capacity: usize,
}

impl FfiBuffer {
    /// Create an empty buffer.
    pub fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    /// Create a buffer from a Vec<u8>.
    pub fn from_vec(mut vec: Vec<u8>) -> Self {
        let buf = Self {
            data: vec.as_mut_ptr(),
            len: vec.len(),
            capacity: vec.capacity(),
        };
        std::mem::forget(vec);
        buf
    }

    /// Create a buffer from bytes (copies data).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_vec(bytes.to_vec())
    }

    /// Convert to a slice (unsafe - caller must ensure validity).
    ///
    /// # Safety
    /// Caller must ensure the buffer is valid and hasn't been freed.
    pub unsafe fn as_slice(&self) -> &[u8] {
        if self.data.is_null() || self.len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(self.data, self.len)
        }
    }
}

/// A C-compatible string (null-terminated or with length).
#[repr(C)]
pub struct FfiString {
    /// Pointer to null-terminated UTF-8 string
    pub ptr: *mut c_char,
    /// Length of string (not including null terminator)
    pub len: usize,
}

impl FfiString {
    /// Create a null/empty string.
    pub fn null() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }

    /// Create from a Rust String.
    pub fn from_string(s: String) -> Self {
        let c_string = match std::ffi::CString::new(s) {
            Ok(cs) => cs,
            Err(_) => return Self::null(),
        };
        let len = c_string.as_bytes().len();
        let ptr = c_string.into_raw();
        Self { ptr, len }
    }

    /// Create from a &str.
    pub fn from_str(s: &str) -> Self {
        Self::from_string(s.to_string())
    }

    /// Convert to a Rust &str (unsafe).
    ///
    /// # Safety
    /// Caller must ensure the string is valid and hasn't been freed.
    pub unsafe fn as_str(&self) -> Option<&str> {
        if self.ptr.is_null() {
            return None;
        }
        let c_str = std::ffi::CStr::from_ptr(self.ptr);
        c_str.to_str().ok()
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Result type for all FFI calls.
/// Contains either success data or error information.
#[repr(C)]
pub struct FfiResult {
    /// True if operation succeeded
    pub success: bool,
    /// Pointer to result data (JSON bytes, null on error)
    pub data: *mut u8,
    /// Length of result data
    pub data_len: usize,
    /// Error code (0 = success, see FfiErrorCode)
    pub error_code: i32,
    /// Error message (null if success)
    pub error_message: *mut c_char,
}

impl FfiResult {
    /// Create a success result with JSON data.
    pub fn success(json: Vec<u8>) -> Self {
        let mut data = json;
        let ptr = data.as_mut_ptr();
        let len = data.len();
        std::mem::forget(data);

        Self {
            success: true,
            data: ptr,
            data_len: len,
            error_code: 0,
            error_message: std::ptr::null_mut(),
        }
    }

    /// Create an error result.
    pub fn error(code: i32, message: &str) -> Self {
        let c_message = std::ffi::CString::new(message)
            .unwrap_or_else(|_| std::ffi::CString::new("Unknown error").unwrap());

        Self {
            success: false,
            data: std::ptr::null_mut(),
            data_len: 0,
            error_code: code,
            error_message: c_message.into_raw(),
        }
    }

    /// Create a null pointer error.
    pub fn null_error() -> Self {
        Self::error(
            super::error::FfiErrorCode::NullPointer as i32,
            "Null pointer provided",
        )
    }

    /// Create a not initialized error.
    pub fn not_initialized() -> Self {
        Self::error(
            super::error::FfiErrorCode::NotInitialized as i32,
            "Bridge not initialized",
        )
    }
}

// ============================================================================
// Request Types (C-compatible)
// ============================================================================

/// Spot price request.
#[repr(C)]
pub struct FfiSpotPriceRequest {
    /// Chain ID (e.g., 1 for Ethereum)
    pub chain_id: u64,
    /// Component/pool ID (null-terminated string)
    pub component_id: *const c_char,
    /// Base token address (null-terminated string)
    pub base_token: *const c_char,
    /// Quote token address (null-terminated string)
    pub quote_token: *const c_char,
}

/// GetAmountOut request.
#[repr(C)]
pub struct FfiGetAmountOutRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID (null-terminated string)
    pub component_id: *const c_char,
    /// Input token address (null-terminated string)
    pub token_in: *const c_char,
    /// Output token address (null-terminated string)
    pub token_out: *const c_char,
    /// Amount in as decimal string (null-terminated)
    pub amount_in: *const c_char,
    /// State ID as UUID string (null = no state override)
    pub state_id: *const c_char,
    /// Whether to save the resulting state
    pub save_state: bool,
}

/// GetFee request.
#[repr(C)]
pub struct FfiGetFeeRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: *const c_char,
    /// Token address
    pub token: *const c_char,
}

/// GetLimits request.
#[repr(C)]
pub struct FfiGetLimitsRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: *const c_char,
    /// Input token address
    pub token_in: *const c_char,
    /// Output token address
    pub token_out: *const c_char,
}

/// SwapToPrice request.
#[repr(C)]
pub struct FfiSwapToPriceRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: *const c_char,
    /// Input token address
    pub token_in: *const c_char,
    /// Output token address
    pub token_out: *const c_char,
    /// Target price numerator (decimal string)
    pub target_price_numerator: *const c_char,
    /// Target price denominator (decimal string)
    pub target_price_denominator: *const c_char,
}

/// QuerySupply request.
#[repr(C)]
pub struct FfiQuerySupplyRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Component/pool ID
    pub component_id: *const c_char,
    /// Input token address
    pub token_in: *const c_char,
    /// Output token address
    pub token_out: *const c_char,
    /// Minimum price numerator (decimal string)
    pub min_price_numerator: *const c_char,
    /// Minimum price denominator (decimal string)
    pub min_price_denominator: *const c_char,
}

/// GetTokens request.
#[repr(C)]
pub struct FfiGetTokensRequest {
    /// Chain ID (0 = all chains)
    pub chain_id: u64,
    /// Maximum number of tokens to return (0 = no limit)
    pub limit: u32,
}

/// GetComponents request.
#[repr(C)]
pub struct FfiGetComponentsRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Protocol type filter (null = all)
    pub protocol_type: *const c_char,
    /// Minimum TVL filter (0 = no filter)
    pub min_tvl: f64,
    /// Maximum number of components (0 = no limit)
    pub limit: u32,
}

/// GetStatus request.
#[repr(C)]
pub struct FfiGetStatusRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Include chain details
    pub include_chain_details: bool,
}

// ============================================================================
// Batch Request Types
// ============================================================================

/// Batch GetAmountOut request.
#[repr(C)]
pub struct FfiBatchGetAmountOutRequest {
    /// Array of individual requests
    pub requests: *const FfiGetAmountOutRequest,
    /// Number of requests
    pub count: usize,
}

/// A single hop in a route.
#[repr(C)]
pub struct FfiRouteHop {
    /// Component/pool ID
    pub component_id: *const c_char,
    /// Input token address
    pub token_in: *const c_char,
    /// Output token address
    pub token_out: *const c_char,
}

/// SimulateRoute request.
#[repr(C)]
pub struct FfiSimulateRouteRequest {
    /// Chain ID
    pub chain_id: u64,
    /// Array of hops
    pub hops: *const FfiRouteHop,
    /// Number of hops
    pub hop_count: usize,
    /// Initial amount in (decimal string)
    pub amount_in: *const c_char,
}

// ============================================================================
// Helper Functions for Request Conversion
// ============================================================================

use num_bigint::BigUint;
use std::ffi::CStr;
use tycho_core::types::request as core_req;

/// Convert a C string pointer to a Rust String.
///
/// # Safety
/// Pointer must be valid and null-terminated.
pub unsafe fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string())
}

/// Convert a C string pointer to a BigUint.
///
/// # Safety
/// Pointer must be valid and null-terminated.
pub unsafe fn cstr_to_biguint(ptr: *const c_char) -> Option<BigUint> {
    cstr_to_string(ptr).and_then(|s| s.parse().ok())
}

impl FfiSpotPriceRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> Option<core_req::SpotPriceRequest> {
        Some(core_req::SpotPriceRequest {
            chain_id: self.chain_id,
            component_id: cstr_to_string(self.component_id)?,
            base_token: cstr_to_string(self.base_token)?,
            quote_token: cstr_to_string(self.quote_token)?,
        })
    }
}

impl FfiGetAmountOutRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> Option<core_req::GetAmountOutRequest> {
        Some(core_req::GetAmountOutRequest {
            chain_id: self.chain_id,
            component_id: cstr_to_string(self.component_id)?,
            token_in: cstr_to_string(self.token_in)?,
            token_out: cstr_to_string(self.token_out)?,
            amount_in: cstr_to_biguint(self.amount_in)?,
            state_id: cstr_to_string(self.state_id).and_then(|s| s.parse().ok()),
            save_state: self.save_state,
        })
    }
}

impl FfiGetFeeRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> Option<core_req::GetFeeRequest> {
        Some(core_req::GetFeeRequest {
            chain_id: self.chain_id,
            component_id: cstr_to_string(self.component_id)?,
            token: cstr_to_string(self.token)?,
        })
    }
}

impl FfiGetLimitsRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> Option<core_req::GetLimitsRequest> {
        Some(core_req::GetLimitsRequest {
            chain_id: self.chain_id,
            component_id: cstr_to_string(self.component_id)?,
            token_in: cstr_to_string(self.token_in)?,
            token_out: cstr_to_string(self.token_out)?,
        })
    }
}

impl FfiSwapToPriceRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> Option<core_req::SwapToPriceRequest> {
        Some(core_req::SwapToPriceRequest {
            chain_id: self.chain_id,
            component_id: cstr_to_string(self.component_id)?,
            token_in: cstr_to_string(self.token_in)?,
            token_out: cstr_to_string(self.token_out)?,
            target_price_numerator: cstr_to_biguint(self.target_price_numerator)?,
            target_price_denominator: cstr_to_biguint(self.target_price_denominator)?,
        })
    }
}

impl FfiQuerySupplyRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> Option<core_req::QuerySupplyRequest> {
        Some(core_req::QuerySupplyRequest {
            chain_id: self.chain_id,
            component_id: cstr_to_string(self.component_id)?,
            token_in: cstr_to_string(self.token_in)?,
            token_out: cstr_to_string(self.token_out)?,
            min_price_numerator: cstr_to_biguint(self.min_price_numerator)?,
            min_price_denominator: cstr_to_biguint(self.min_price_denominator)?,
        })
    }
}

impl FfiGetTokensRequest {
    /// Convert to core request type.
    pub fn to_core(&self) -> core_req::GetTokensRequest {
        core_req::GetTokensRequest {
            chain_id: if self.chain_id == 0 {
                None
            } else {
                Some(self.chain_id)
            },
            addresses: vec![],
            limit: if self.limit == 0 {
                None
            } else {
                Some(self.limit)
            },
        }
    }
}

impl FfiGetComponentsRequest {
    /// Convert to core request type.
    ///
    /// # Safety
    /// All string pointers must be valid.
    pub unsafe fn to_core(&self) -> core_req::GetProtocolComponentsRequest {
        core_req::GetProtocolComponentsRequest {
            chain_id: self.chain_id,
            component_ids: vec![],
            min_tvl: if self.min_tvl > 0.0 {
                Some(self.min_tvl)
            } else {
                None
            },
            protocol_type: cstr_to_string(self.protocol_type),
            limit: if self.limit == 0 {
                None
            } else {
                Some(self.limit)
            },
        }
    }
}

impl FfiGetStatusRequest {
    /// Convert to core request type.
    pub fn to_core(&self) -> core_req::GetStatusRequest {
        core_req::GetStatusRequest {
            chain_id: self.chain_id,
            include_chain_details: self.include_chain_details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // ========================================================================
    // FfiBuffer Tests
    // ========================================================================

    #[test]
    fn test_ffi_buffer_empty() {
        let buf = FfiBuffer::empty();
        assert!(buf.data.is_null());
        assert_eq!(buf.len, 0);
        assert_eq!(buf.capacity, 0);
    }

    #[test]
    fn test_ffi_buffer_from_vec() {
        let data = vec![1u8, 2, 3, 4, 5];
        let buf = FfiBuffer::from_vec(data);
        assert!(!buf.data.is_null());
        assert_eq!(buf.len, 5);
        assert!(buf.capacity >= 5);

        // Verify data content
        unsafe {
            let slice = buf.as_slice();
            assert_eq!(slice, &[1, 2, 3, 4, 5]);
        }

        // Clean up (reconstruct vec to free memory)
        unsafe {
            let _ = Vec::from_raw_parts(buf.data, buf.len, buf.capacity);
        }
    }

    #[test]
    fn test_ffi_buffer_from_bytes() {
        let data = b"hello world";
        let buf = FfiBuffer::from_bytes(data);
        assert!(!buf.data.is_null());
        assert_eq!(buf.len, 11);

        unsafe {
            let slice = buf.as_slice();
            assert_eq!(slice, b"hello world");
            // Clean up
            let _ = Vec::from_raw_parts(buf.data, buf.len, buf.capacity);
        }
    }

    #[test]
    fn test_ffi_buffer_as_slice_empty() {
        let buf = FfiBuffer::empty();
        unsafe {
            let slice = buf.as_slice();
            assert!(slice.is_empty());
        }
    }

    // ========================================================================
    // FfiString Tests
    // ========================================================================

    #[test]
    fn test_ffi_string_null() {
        let s = FfiString::null();
        assert!(s.ptr.is_null());
        assert_eq!(s.len, 0);
    }

    #[test]
    fn test_ffi_string_from_string() {
        let s = FfiString::from_string("hello".to_string());
        assert!(!s.ptr.is_null());
        assert_eq!(s.len, 5);

        unsafe {
            let str_val = s.as_str();
            assert_eq!(str_val, Some("hello"));
            // Clean up
            let _ = CString::from_raw(s.ptr);
        }
    }

    #[test]
    fn test_ffi_string_from_str() {
        let s = FfiString::from_str("world");
        assert!(!s.ptr.is_null());
        assert_eq!(s.len, 5);

        unsafe {
            assert_eq!(s.as_str(), Some("world"));
            let _ = CString::from_raw(s.ptr);
        }
    }

    #[test]
    fn test_ffi_string_with_null_byte() {
        // CString::new fails if the string contains a null byte
        let s = FfiString::from_string("hello\0world".to_string());
        // Should return null since the string contains embedded null
        assert!(s.ptr.is_null());
        assert_eq!(s.len, 0);
    }

    #[test]
    fn test_ffi_string_empty() {
        let s = FfiString::from_str("");
        assert!(!s.ptr.is_null());
        assert_eq!(s.len, 0);

        unsafe {
            assert_eq!(s.as_str(), Some(""));
            let _ = CString::from_raw(s.ptr);
        }
    }

    // ========================================================================
    // FfiResult Tests
    // ========================================================================

    #[test]
    fn test_ffi_result_success() {
        let json = b"{\"status\": \"ok\"}".to_vec();
        let result = FfiResult::success(json);

        assert!(result.success);
        assert!(!result.data.is_null());
        assert_eq!(result.data_len, 16);
        assert_eq!(result.error_code, 0);
        assert!(result.error_message.is_null());

        // Clean up
        unsafe {
            let _ = Vec::from_raw_parts(result.data, result.data_len, result.data_len);
        }
    }

    #[test]
    fn test_ffi_result_error() {
        let result = FfiResult::error(42, "Something went wrong");

        assert!(!result.success);
        assert!(result.data.is_null());
        assert_eq!(result.data_len, 0);
        assert_eq!(result.error_code, 42);
        assert!(!result.error_message.is_null());

        // Verify error message
        unsafe {
            let msg = CStr::from_ptr(result.error_message);
            assert_eq!(msg.to_str().unwrap(), "Something went wrong");
            // Clean up
            let _ = CString::from_raw(result.error_message);
        }
    }

    #[test]
    fn test_ffi_result_null_error() {
        let result = FfiResult::null_error();

        assert!(!result.success);
        assert_eq!(
            result.error_code,
            crate::error::FfiErrorCode::NullPointer as i32
        );

        // Clean up
        if !result.error_message.is_null() {
            unsafe {
                let _ = CString::from_raw(result.error_message);
            }
        }
    }

    #[test]
    fn test_ffi_result_not_initialized() {
        let result = FfiResult::not_initialized();

        assert!(!result.success);
        assert_eq!(
            result.error_code,
            crate::error::FfiErrorCode::NotInitialized as i32
        );

        // Clean up
        if !result.error_message.is_null() {
            unsafe {
                let _ = CString::from_raw(result.error_message);
            }
        }
    }

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_cstr_to_string_valid() {
        let c_str = CString::new("test string").unwrap();
        unsafe {
            let result = cstr_to_string(c_str.as_ptr());
            assert_eq!(result, Some("test string".to_string()));
        }
    }

    #[test]
    fn test_cstr_to_string_null() {
        unsafe {
            let result = cstr_to_string(std::ptr::null());
            assert_eq!(result, None);
        }
    }

    #[test]
    fn test_cstr_to_biguint_valid() {
        let c_str = CString::new("123456789").unwrap();
        unsafe {
            let result = cstr_to_biguint(c_str.as_ptr());
            assert!(result.is_some());
            assert_eq!(result.unwrap().to_string(), "123456789");
        }
    }

    #[test]
    fn test_cstr_to_biguint_invalid() {
        let c_str = CString::new("not_a_number").unwrap();
        unsafe {
            let result = cstr_to_biguint(c_str.as_ptr());
            assert_eq!(result, None);
        }
    }

    #[test]
    fn test_cstr_to_biguint_null() {
        unsafe {
            let result = cstr_to_biguint(std::ptr::null());
            assert_eq!(result, None);
        }
    }

    // ========================================================================
    // Request Conversion Tests
    // ========================================================================

    #[test]
    fn test_spot_price_request_to_core() {
        let component_id = CString::new("pool_123").unwrap();
        let base_token = CString::new("0xtoken_a").unwrap();
        let quote_token = CString::new("0xtoken_b").unwrap();

        let request = FfiSpotPriceRequest {
            chain_id: 1,
            component_id: component_id.as_ptr(),
            base_token: base_token.as_ptr(),
            quote_token: quote_token.as_ptr(),
        };

        unsafe {
            let core_req = request.to_core();
            assert!(core_req.is_some());
            let core_req = core_req.unwrap();
            assert_eq!(core_req.chain_id, 1);
            assert_eq!(core_req.component_id, "pool_123");
            assert_eq!(core_req.base_token, "0xtoken_a");
            assert_eq!(core_req.quote_token, "0xtoken_b");
        }
    }

    #[test]
    fn test_spot_price_request_to_core_null_field() {
        let component_id = CString::new("pool_123").unwrap();
        let base_token = CString::new("0xtoken_a").unwrap();

        let request = FfiSpotPriceRequest {
            chain_id: 1,
            component_id: component_id.as_ptr(),
            base_token: base_token.as_ptr(),
            quote_token: std::ptr::null(), // null field
        };

        unsafe {
            let core_req = request.to_core();
            assert!(core_req.is_none());
        }
    }

    #[test]
    fn test_get_tokens_request_to_core() {
        let request = FfiGetTokensRequest {
            chain_id: 0,
            limit: 0,
        };

        let core_req = request.to_core();
        assert_eq!(core_req.chain_id, None);
        assert_eq!(core_req.limit, None);

        let request_with_values = FfiGetTokensRequest {
            chain_id: 1,
            limit: 100,
        };

        let core_req = request_with_values.to_core();
        assert_eq!(core_req.chain_id, Some(1));
        assert_eq!(core_req.limit, Some(100));
    }

    #[test]
    fn test_get_status_request_to_core() {
        let request = FfiGetStatusRequest {
            chain_id: 42,
            include_chain_details: true,
        };

        let core_req = request.to_core();
        assert_eq!(core_req.chain_id, 42);
        assert!(core_req.include_chain_details);
    }
}
