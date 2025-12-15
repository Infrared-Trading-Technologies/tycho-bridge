//! FFI memory management.
//!
//! Provides functions for allocating and freeing memory across the FFI boundary.
//! Memory allocated by Rust must be freed by Rust; these functions provide
//! the necessary interface for foreign code to request deallocation.

use std::ffi::{c_char, CString};

use crate::types::{FfiBuffer, FfiResult, FfiString};

// ============================================================================
// Memory Allocation Functions
// ============================================================================

/// Allocate a buffer of the specified size.
///
/// # Arguments
/// * `size` - Number of bytes to allocate
///
/// # Returns
/// A buffer with allocated memory, or empty buffer on failure.
///
/// # Safety
/// The returned buffer must be freed with `tycho_free_buffer`.
#[no_mangle]
pub extern "C" fn tycho_alloc(size: usize) -> FfiBuffer {
    if size == 0 {
        return FfiBuffer::empty();
    }

    let vec: Vec<u8> = vec![0; size];
    FfiBuffer::from_vec(vec)
}

/// Free a buffer allocated by Rust.
///
/// # Arguments
/// * `buffer` - The buffer to free
///
/// # Safety
/// The buffer must have been allocated by `tycho_alloc` or returned from
/// a tycho FFI function. Double-free will cause undefined behavior.
#[no_mangle]
pub extern "C" fn tycho_free_buffer(buffer: FfiBuffer) {
    if buffer.data.is_null() {
        return;
    }

    unsafe {
        // Reconstruct the Vec and let it drop
        let _ = Vec::from_raw_parts(buffer.data, buffer.len, buffer.capacity);
    }
}

/// Free a string allocated by Rust.
///
/// # Arguments
/// * `string` - The string to free
///
/// # Safety
/// The string must have been allocated by Rust FFI functions.
/// Double-free will cause undefined behavior.
#[no_mangle]
pub extern "C" fn tycho_free_string(string: FfiString) {
    if string.ptr.is_null() {
        return;
    }

    unsafe {
        // Reconstruct the CString and let it drop
        let _ = CString::from_raw(string.ptr);
    }
}

/// Free an FfiResult and all its associated memory.
///
/// # Arguments
/// * `result` - The result to free
///
/// # Safety
/// The result must have been returned by a tycho FFI function.
/// Double-free will cause undefined behavior.
#[no_mangle]
pub extern "C" fn tycho_free_result(result: FfiResult) {
    // Free data buffer
    if !result.data.is_null() && result.data_len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(result.data, result.data_len, result.data_len);
        }
    }

    // Free error message
    if !result.error_message.is_null() {
        unsafe {
            let _ = CString::from_raw(result.error_message);
        }
    }
}

// ============================================================================
// String Utilities
// ============================================================================

/// Duplicate a C string.
///
/// # Arguments
/// * `ptr` - Pointer to null-terminated string
///
/// # Returns
/// New FfiString that must be freed with `tycho_free_string`, or null on failure.
///
/// # Safety
/// Input pointer must be valid and null-terminated.
#[no_mangle]
pub unsafe extern "C" fn tycho_strdup(ptr: *const c_char) -> FfiString {
    if ptr.is_null() {
        return FfiString::null();
    }

    match std::ffi::CStr::from_ptr(ptr).to_str() {
        Ok(s) => FfiString::from_str(s),
        Err(_) => FfiString::null(),
    }
}

/// Get the length of a buffer.
#[no_mangle]
pub extern "C" fn tycho_buffer_len(buffer: *const FfiBuffer) -> usize {
    if buffer.is_null() {
        return 0;
    }
    unsafe { (*buffer).len }
}

/// Get the data pointer from a buffer.
#[no_mangle]
pub extern "C" fn tycho_buffer_data(buffer: *const FfiBuffer) -> *const u8 {
    if buffer.is_null() {
        return std::ptr::null();
    }
    unsafe { (*buffer).data }
}

/// Get the length of a string (not including null terminator).
#[no_mangle]
pub extern "C" fn tycho_string_len(string: *const FfiString) -> usize {
    if string.is_null() {
        return 0;
    }
    unsafe { (*string).len }
}

/// Get the data pointer from a string.
#[no_mangle]
pub extern "C" fn tycho_string_ptr(string: *const FfiString) -> *const c_char {
    if string.is_null() {
        return std::ptr::null();
    }
    unsafe { (*string).ptr }
}

// ============================================================================
// JSON Utilities
// ============================================================================

/// Parse JSON data from a result.
/// This is a helper for languages that need to extract the JSON string.
///
/// # Returns
/// A new string containing the JSON data, or null on failure.
/// Must be freed with `tycho_free_string`.
#[no_mangle]
pub extern "C" fn tycho_result_to_json_string(result: *const FfiResult) -> FfiString {
    if result.is_null() {
        return FfiString::null();
    }

    let result = unsafe { &*result };

    if !result.success || result.data.is_null() || result.data_len == 0 {
        return FfiString::null();
    }

    let slice = unsafe { std::slice::from_raw_parts(result.data, result.data_len) };

    match std::str::from_utf8(slice) {
        Ok(s) => FfiString::from_str(s),
        Err(_) => FfiString::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    // ========================================================================
    // Allocation Tests
    // ========================================================================

    #[test]
    fn test_tycho_alloc_zero() {
        let buf = tycho_alloc(0);
        assert!(buf.data.is_null());
        assert_eq!(buf.len, 0);
        assert_eq!(buf.capacity, 0);
    }

    #[test]
    fn test_tycho_alloc_nonzero() {
        let buf = tycho_alloc(100);
        assert!(!buf.data.is_null());
        assert_eq!(buf.len, 100);
        assert!(buf.capacity >= 100);

        // Verify memory is zero-initialized
        unsafe {
            let slice = std::slice::from_raw_parts(buf.data, buf.len);
            assert!(slice.iter().all(|&b| b == 0));
        }

        // Free the buffer
        tycho_free_buffer(buf);
    }

    #[test]
    fn test_tycho_free_buffer_null() {
        // Should not panic on null
        let buf = FfiBuffer::empty();
        tycho_free_buffer(buf);
    }

    #[test]
    fn test_tycho_free_buffer_valid() {
        let buf = tycho_alloc(50);
        assert!(!buf.data.is_null());
        tycho_free_buffer(buf);
        // No way to verify memory is freed, but should not panic
    }

    // ========================================================================
    // String Tests
    // ========================================================================

    #[test]
    fn test_tycho_free_string_null() {
        // Should not panic on null
        let s = FfiString::null();
        tycho_free_string(s);
    }

    #[test]
    fn test_tycho_free_string_valid() {
        let s = FfiString::from_str("hello world");
        assert!(!s.ptr.is_null());
        tycho_free_string(s);
    }

    #[test]
    fn test_tycho_strdup_null() {
        unsafe {
            let s = tycho_strdup(std::ptr::null());
            assert!(s.ptr.is_null());
            assert_eq!(s.len, 0);
        }
    }

    #[test]
    fn test_tycho_strdup_valid() {
        let original = CString::new("duplicate me").unwrap();
        unsafe {
            let dup = tycho_strdup(original.as_ptr());
            assert!(!dup.ptr.is_null());
            assert_eq!(dup.len, 12);

            let dup_str = CStr::from_ptr(dup.ptr);
            assert_eq!(dup_str.to_str().unwrap(), "duplicate me");

            tycho_free_string(dup);
        }
    }

    // ========================================================================
    // Buffer Accessor Tests
    // ========================================================================

    #[test]
    fn test_tycho_buffer_len_null() {
        assert_eq!(tycho_buffer_len(std::ptr::null()), 0);
    }

    #[test]
    fn test_tycho_buffer_len_valid() {
        let buf = tycho_alloc(42);
        assert_eq!(tycho_buffer_len(&buf), 42);
        tycho_free_buffer(buf);
    }

    #[test]
    fn test_tycho_buffer_data_null() {
        assert!(tycho_buffer_data(std::ptr::null()).is_null());
    }

    #[test]
    fn test_tycho_buffer_data_valid() {
        let buf = tycho_alloc(10);
        let data_ptr = tycho_buffer_data(&buf);
        assert_eq!(data_ptr, buf.data as *const u8);
        tycho_free_buffer(buf);
    }

    // ========================================================================
    // String Accessor Tests
    // ========================================================================

    #[test]
    fn test_tycho_string_len_null() {
        assert_eq!(tycho_string_len(std::ptr::null()), 0);
    }

    #[test]
    fn test_tycho_string_len_valid() {
        let s = FfiString::from_str("test");
        assert_eq!(tycho_string_len(&s), 4);
        tycho_free_string(s);
    }

    #[test]
    fn test_tycho_string_ptr_null() {
        assert!(tycho_string_ptr(std::ptr::null()).is_null());
    }

    #[test]
    fn test_tycho_string_ptr_valid() {
        let s = FfiString::from_str("pointer");
        let ptr = tycho_string_ptr(&s);
        assert_eq!(ptr, s.ptr as *const c_char);
        tycho_free_string(s);
    }

    // ========================================================================
    // Result Tests
    // ========================================================================

    #[test]
    fn test_tycho_free_result_success() {
        let json = b"{\"key\": \"value\"}".to_vec();
        let result = FfiResult::success(json);
        assert!(result.success);
        tycho_free_result(result);
    }

    #[test]
    fn test_tycho_free_result_error() {
        let result = FfiResult::error(42, "Test error");
        assert!(!result.success);
        tycho_free_result(result);
    }

    #[test]
    fn test_tycho_free_result_null_pointers() {
        // Result with null data and null error message
        let result = FfiResult {
            success: true,
            data: std::ptr::null_mut(),
            data_len: 0,
            error_code: 0,
            error_message: std::ptr::null_mut(),
        };
        tycho_free_result(result);
    }

    // ========================================================================
    // JSON Utility Tests
    // ========================================================================

    #[test]
    fn test_tycho_result_to_json_string_null() {
        let s = tycho_result_to_json_string(std::ptr::null());
        assert!(s.ptr.is_null());
    }

    #[test]
    fn test_tycho_result_to_json_string_success() {
        let json = b"{\"status\":\"ok\"}".to_vec();
        let result = FfiResult::success(json);

        let json_str = tycho_result_to_json_string(&result);
        assert!(!json_str.ptr.is_null());

        unsafe {
            let s = json_str.as_str();
            assert_eq!(s, Some("{\"status\":\"ok\"}"));
        }

        tycho_free_string(json_str);
        tycho_free_result(result);
    }

    #[test]
    fn test_tycho_result_to_json_string_error() {
        let result = FfiResult::error(1, "error message");

        let json_str = tycho_result_to_json_string(&result);
        // Should return null for error results
        assert!(json_str.ptr.is_null());

        tycho_free_result(result);
    }

    #[test]
    fn test_tycho_result_to_json_string_empty_data() {
        let result = FfiResult {
            success: true,
            data: std::ptr::null_mut(),
            data_len: 0,
            error_code: 0,
            error_message: std::ptr::null_mut(),
        };

        let json_str = tycho_result_to_json_string(&result);
        assert!(json_str.ptr.is_null());
    }
}
