//! FFI error handling.
//!
//! Provides error codes and thread-local last-error mechanism for FFI callers.
//! This allows callers to retrieve detailed error information after a failed call.

use std::cell::RefCell;
use std::ffi::{c_char, CString};

use tycho_core::BridgeError;

/// Error codes for FFI callers.
/// These match the codes in BridgeError::code().
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiErrorCode {
    /// No error
    Ok = 0,
    /// Component not found
    ComponentNotFound = 1,
    /// Token not found
    TokenNotFound = 2,
    /// Invalid argument provided
    InvalidArgument = 3,
    /// Simulation failed
    SimulationError = 4,
    /// State unavailable
    StateUnavailable = 5,
    /// Chain not supported
    UnsupportedChain = 6,
    /// Configuration error
    ConfigError = 7,
    /// Connection error
    ConnectionError = 8,
    /// Null pointer provided
    NullPointer = 50,
    /// Invalid UTF-8 string
    InvalidUtf8 = 51,
    /// Bridge not initialized
    NotInitialized = 52,
    /// JSON serialization error
    JsonError = 53,
    /// Internal error
    InternalError = 99,
}

impl From<&BridgeError> for FfiErrorCode {
    fn from(err: &BridgeError) -> Self {
        match err {
            BridgeError::ComponentNotFound { .. } => FfiErrorCode::ComponentNotFound,
            BridgeError::TokenNotFound { .. } => FfiErrorCode::TokenNotFound,
            BridgeError::InvalidArgument { .. } => FfiErrorCode::InvalidArgument,
            BridgeError::SimulationError { .. } => FfiErrorCode::SimulationError,
            BridgeError::StateUnavailable { .. } => FfiErrorCode::StateUnavailable,
            BridgeError::ChainNotSupported { .. } => FfiErrorCode::UnsupportedChain,
            BridgeError::ConfigError { .. } => FfiErrorCode::ConfigError,
            BridgeError::ConnectionError { .. } => FfiErrorCode::ConnectionError,
            BridgeError::Internal { .. } => FfiErrorCode::InternalError,
        }
    }
}

// ============================================================================
// Thread-Local Last Error Storage
// ============================================================================

// Thread-local storage for the last error.
thread_local! {
    static LAST_ERROR: RefCell<Option<LastError>> = const { RefCell::new(None) };
}

/// Stored error information.
struct LastError {
    code: FfiErrorCode,
    message: String,
}

/// Set the last error for the current thread.
pub(crate) fn set_last_error(code: FfiErrorCode, message: impl Into<String>) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(LastError {
            code,
            message: message.into(),
        });
    });
}

/// Set the last error from a BridgeError.
pub(crate) fn set_last_bridge_error(err: &BridgeError) {
    set_last_error(FfiErrorCode::from(err), err.to_string());
}

/// Clear the last error for the current thread.
pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

// ============================================================================
// FFI Error Retrieval Functions
// ============================================================================

/// Get the last error code for the current thread.
///
/// # Returns
/// The error code from the last failed FFI call, or 0 if no error.
#[no_mangle]
pub extern "C" fn tycho_last_error_code() -> i32 {
    LAST_ERROR.with(|cell| cell.borrow().as_ref().map(|e| e.code as i32).unwrap_or(0))
}

/// Get the last error message for the current thread.
///
/// # Returns
/// A pointer to a null-terminated error message, or null if no error.
/// The returned string is valid until the next FFI call on this thread.
///
/// # Safety
/// The returned pointer is only valid until the next tycho FFI call.
/// The caller should NOT free this pointer.
#[no_mangle]
pub extern "C" fn tycho_last_error_message() -> *const c_char {
    thread_local! {
        static CACHED_MESSAGE: RefCell<Option<CString>> = const { RefCell::new(None) };
    }

    LAST_ERROR.with(|error_cell| {
        let error = error_cell.borrow();
        let message = error.as_ref().map(|e| e.message.as_str()).unwrap_or("");

        CACHED_MESSAGE.with(|cache_cell| {
            let c_string = CString::new(message).unwrap_or_else(|_| CString::new("").unwrap());
            let ptr = c_string.as_ptr();
            *cache_cell.borrow_mut() = Some(c_string);
            ptr
        })
    })
}

/// Get the last error message length (not including null terminator).
///
/// # Returns
/// The length of the last error message, or 0 if no error.
#[no_mangle]
pub extern "C" fn tycho_last_error_length() -> usize {
    LAST_ERROR.with(|cell| cell.borrow().as_ref().map(|e| e.message.len()).unwrap_or(0))
}

/// Clear the last error for the current thread.
#[no_mangle]
pub extern "C" fn tycho_clear_last_error() {
    clear_last_error();
}

/// Check if there is a pending error.
///
/// # Returns
/// True if there is a pending error, false otherwise.
#[no_mangle]
pub extern "C" fn tycho_has_error() -> bool {
    LAST_ERROR.with(|cell| cell.borrow().is_some())
}

// ============================================================================
// Error Description Functions
// ============================================================================

/// Get a human-readable description of an error code.
///
/// # Arguments
/// * `code` - The error code
///
/// # Returns
/// A static string describing the error code.
#[no_mangle]
pub extern "C" fn tycho_error_code_description(code: i32) -> *const c_char {
    static OK_DESC: &[u8] = b"Success\0";
    static COMPONENT_NOT_FOUND_DESC: &[u8] = b"Component not found\0";
    static TOKEN_NOT_FOUND_DESC: &[u8] = b"Token not found\0";
    static INVALID_ARGUMENT_DESC: &[u8] = b"Invalid argument\0";
    static SIMULATION_ERROR_DESC: &[u8] = b"Simulation failed\0";
    static STATE_UNAVAILABLE_DESC: &[u8] = b"State unavailable\0";
    static UNSUPPORTED_CHAIN_DESC: &[u8] = b"Chain not supported\0";
    static CONFIG_ERROR_DESC: &[u8] = b"Configuration error\0";
    static CONNECTION_ERROR_DESC: &[u8] = b"Connection error\0";
    static NULL_POINTER_DESC: &[u8] = b"Null pointer provided\0";
    static INVALID_UTF8_DESC: &[u8] = b"Invalid UTF-8 string\0";
    static NOT_INITIALIZED_DESC: &[u8] = b"Bridge not initialized\0";
    static JSON_ERROR_DESC: &[u8] = b"JSON serialization error\0";
    static INTERNAL_ERROR_DESC: &[u8] = b"Internal error\0";
    static UNKNOWN_DESC: &[u8] = b"Unknown error\0";

    let desc = match code {
        0 => OK_DESC,
        1 => COMPONENT_NOT_FOUND_DESC,
        2 => TOKEN_NOT_FOUND_DESC,
        3 => INVALID_ARGUMENT_DESC,
        4 => SIMULATION_ERROR_DESC,
        5 => STATE_UNAVAILABLE_DESC,
        6 => UNSUPPORTED_CHAIN_DESC,
        7 => CONFIG_ERROR_DESC,
        8 => CONNECTION_ERROR_DESC,
        50 => NULL_POINTER_DESC,
        51 => INVALID_UTF8_DESC,
        52 => NOT_INITIALIZED_DESC,
        53 => JSON_ERROR_DESC,
        99 => INTERNAL_ERROR_DESC,
        _ => UNKNOWN_DESC,
    };

    desc.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    // ========================================================================
    // FfiErrorCode Tests
    // ========================================================================

    #[test]
    fn test_error_code_values() {
        // Verify error codes have expected integer values
        assert_eq!(FfiErrorCode::Ok as i32, 0);
        assert_eq!(FfiErrorCode::ComponentNotFound as i32, 1);
        assert_eq!(FfiErrorCode::TokenNotFound as i32, 2);
        assert_eq!(FfiErrorCode::InvalidArgument as i32, 3);
        assert_eq!(FfiErrorCode::SimulationError as i32, 4);
        assert_eq!(FfiErrorCode::StateUnavailable as i32, 5);
        assert_eq!(FfiErrorCode::UnsupportedChain as i32, 6);
        assert_eq!(FfiErrorCode::ConfigError as i32, 7);
        assert_eq!(FfiErrorCode::ConnectionError as i32, 8);
        assert_eq!(FfiErrorCode::NullPointer as i32, 50);
        assert_eq!(FfiErrorCode::InvalidUtf8 as i32, 51);
        assert_eq!(FfiErrorCode::NotInitialized as i32, 52);
        assert_eq!(FfiErrorCode::JsonError as i32, 53);
        assert_eq!(FfiErrorCode::InternalError as i32, 99);
    }

    #[test]
    fn test_error_code_from_bridge_error() {
        // Test conversion from BridgeError to FfiErrorCode
        let component_err = BridgeError::ComponentNotFound {
            component_id: "test".into(),
        };
        assert_eq!(
            FfiErrorCode::from(&component_err),
            FfiErrorCode::ComponentNotFound
        );

        let token_err = BridgeError::TokenNotFound {
            token: "0x123".into(),
        };
        assert_eq!(FfiErrorCode::from(&token_err), FfiErrorCode::TokenNotFound);

        let chain_err = BridgeError::ChainNotSupported { chain_id: 999 };
        assert_eq!(
            FfiErrorCode::from(&chain_err),
            FfiErrorCode::UnsupportedChain
        );
    }

    // ========================================================================
    // Thread-Local Error Storage Tests
    // ========================================================================

    #[test]
    fn test_set_and_get_last_error() {
        // Clear any existing error first
        clear_last_error();
        assert!(!tycho_has_error());
        assert_eq!(tycho_last_error_code(), 0);

        // Set an error
        set_last_error(FfiErrorCode::InvalidArgument, "Test error message");

        // Verify error is set
        assert!(tycho_has_error());
        assert_eq!(
            tycho_last_error_code(),
            FfiErrorCode::InvalidArgument as i32
        );
        assert_eq!(tycho_last_error_length(), 18); // "Test error message".len()

        // Get error message
        let msg_ptr = tycho_last_error_message();
        assert!(!msg_ptr.is_null());
        unsafe {
            let msg = CStr::from_ptr(msg_ptr);
            assert_eq!(msg.to_str().unwrap(), "Test error message");
        }

        // Clear the error
        tycho_clear_last_error();
        assert!(!tycho_has_error());
        assert_eq!(tycho_last_error_code(), 0);
        assert_eq!(tycho_last_error_length(), 0);
    }

    #[test]
    fn test_set_last_bridge_error() {
        clear_last_error();

        let bridge_err = BridgeError::SimulationError {
            message: "Simulation failed due to insufficient liquidity".into(),
        };
        set_last_bridge_error(&bridge_err);

        assert!(tycho_has_error());
        assert_eq!(
            tycho_last_error_code(),
            FfiErrorCode::SimulationError as i32
        );

        let msg_ptr = tycho_last_error_message();
        unsafe {
            let msg = CStr::from_ptr(msg_ptr);
            assert!(msg.to_str().unwrap().contains("Simulation failed"));
        }

        clear_last_error();
    }

    #[test]
    fn test_error_overwrite() {
        clear_last_error();

        // Set first error
        set_last_error(FfiErrorCode::TokenNotFound, "First error");
        assert_eq!(tycho_last_error_code(), FfiErrorCode::TokenNotFound as i32);

        // Set second error - should overwrite
        set_last_error(FfiErrorCode::ConfigError, "Second error");
        assert_eq!(tycho_last_error_code(), FfiErrorCode::ConfigError as i32);

        let msg_ptr = tycho_last_error_message();
        unsafe {
            let msg = CStr::from_ptr(msg_ptr);
            assert_eq!(msg.to_str().unwrap(), "Second error");
        }

        clear_last_error();
    }

    #[test]
    fn test_no_error_message() {
        clear_last_error();

        let msg_ptr = tycho_last_error_message();
        assert!(!msg_ptr.is_null()); // Returns empty string, not null
        unsafe {
            let msg = CStr::from_ptr(msg_ptr);
            assert_eq!(msg.to_str().unwrap(), "");
        }
    }

    // ========================================================================
    // Error Description Tests
    // ========================================================================

    #[test]
    fn test_error_code_description() {
        // Test known error codes
        let desc = tycho_error_code_description(0);
        unsafe {
            assert_eq!(CStr::from_ptr(desc).to_str().unwrap(), "Success");
        }

        let desc = tycho_error_code_description(1);
        unsafe {
            assert_eq!(
                CStr::from_ptr(desc).to_str().unwrap(),
                "Component not found"
            );
        }

        let desc = tycho_error_code_description(50);
        unsafe {
            assert_eq!(
                CStr::from_ptr(desc).to_str().unwrap(),
                "Null pointer provided"
            );
        }

        let desc = tycho_error_code_description(99);
        unsafe {
            assert_eq!(CStr::from_ptr(desc).to_str().unwrap(), "Internal error");
        }
    }

    #[test]
    fn test_error_code_description_unknown() {
        // Test unknown error code
        let desc = tycho_error_code_description(9999);
        unsafe {
            assert_eq!(CStr::from_ptr(desc).to_str().unwrap(), "Unknown error");
        }

        // Test negative codes
        let desc = tycho_error_code_description(-1);
        unsafe {
            assert_eq!(CStr::from_ptr(desc).to_str().unwrap(), "Unknown error");
        }
    }

    #[test]
    fn test_all_error_descriptions_are_valid() {
        // Verify all known error codes have valid descriptions
        let known_codes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 50, 51, 52, 53, 99];

        for code in known_codes {
            let desc = tycho_error_code_description(code);
            assert!(!desc.is_null());
            unsafe {
                let desc_str = CStr::from_ptr(desc).to_str();
                assert!(desc_str.is_ok());
                assert!(!desc_str.unwrap().is_empty());
            }
        }
    }
}
