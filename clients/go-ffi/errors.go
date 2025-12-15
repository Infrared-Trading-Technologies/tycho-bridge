package tychoffi

import "fmt"

// ErrorCode represents an error code from the FFI layer.
// These must match the FfiErrorCode enum in tycho-ffi/src/error.rs
type ErrorCode int32

const (
	// ErrorNone indicates no error.
	ErrorNone ErrorCode = 0

	// ErrorComponentNotFound indicates component was not found.
	ErrorComponentNotFound ErrorCode = 1

	// ErrorTokenNotFound indicates token was not found.
	ErrorTokenNotFound ErrorCode = 2

	// ErrorInvalidArgument indicates invalid argument.
	ErrorInvalidArgument ErrorCode = 3

	// ErrorSimulationFailed indicates simulation failed.
	ErrorSimulationFailed ErrorCode = 4

	// ErrorStateUnavailable indicates state is unavailable.
	ErrorStateUnavailable ErrorCode = 5

	// ErrorChainNotSupported indicates chain is not supported.
	ErrorChainNotSupported ErrorCode = 6

	// ErrorConfigError indicates configuration error.
	ErrorConfigError ErrorCode = 7

	// ErrorConnectionError indicates connection error.
	ErrorConnectionError ErrorCode = 8

	// ErrorNullPointer indicates a null pointer was passed.
	ErrorNullPointer ErrorCode = 50

	// ErrorInvalidUtf8 indicates invalid UTF-8 string.
	ErrorInvalidUtf8 ErrorCode = 51

	// ErrorNotInitialized indicates bridge is not initialized.
	ErrorNotInitialized ErrorCode = 52

	// ErrorJsonError indicates JSON serialization error.
	ErrorJsonError ErrorCode = 53

	// ErrorInternalError indicates an internal error.
	ErrorInternalError ErrorCode = 99
)

// String returns a human-readable description of the error code.
func (e ErrorCode) String() string {
	switch e {
	case ErrorNone:
		return "no error"
	case ErrorComponentNotFound:
		return "component not found"
	case ErrorTokenNotFound:
		return "token not found"
	case ErrorInvalidArgument:
		return "invalid argument"
	case ErrorSimulationFailed:
		return "simulation failed"
	case ErrorStateUnavailable:
		return "state unavailable"
	case ErrorChainNotSupported:
		return "chain not supported"
	case ErrorConfigError:
		return "configuration error"
	case ErrorConnectionError:
		return "connection error"
	case ErrorNullPointer:
		return "null pointer"
	case ErrorInvalidUtf8:
		return "invalid UTF-8 string"
	case ErrorNotInitialized:
		return "bridge not initialized"
	case ErrorJsonError:
		return "JSON serialization error"
	case ErrorInternalError:
		return "internal error"
	default:
		return fmt.Sprintf("unknown error code: %d", e)
	}
}

// BridgeError represents an error from the FFI bridge.
type BridgeError struct {
	Code    ErrorCode
	Message string
}

// Error implements the error interface.
func (e *BridgeError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("%s: %s", e.Code, e.Message)
	}
	return e.Code.String()
}

// Is checks if this error matches the target error code.
func (e *BridgeError) Is(target error) bool {
	if t, ok := target.(*BridgeError); ok {
		return e.Code == t.Code
	}
	return false
}

// NewBridgeError creates a new BridgeError from a code and message.
func NewBridgeError(code int32, message string) *BridgeError {
	return &BridgeError{
		Code:    ErrorCode(code),
		Message: message,
	}
}

// Common errors for convenience.
var (
	ErrNilBridge         = &BridgeError{Code: ErrorNullPointer, Message: "bridge is nil"}
	ErrBridgeClosed      = &BridgeError{Code: ErrorNullPointer, Message: "bridge is closed"}
	ErrNotInitialized    = &BridgeError{Code: ErrorNotInitialized, Message: "bridge not initialized"}
	ErrComponentNotFound = &BridgeError{Code: ErrorComponentNotFound}
	ErrTokenNotFound     = &BridgeError{Code: ErrorTokenNotFound}
	ErrInvalidArgument   = &BridgeError{Code: ErrorInvalidArgument}
	ErrSimulationFailed  = &BridgeError{Code: ErrorSimulationFailed}
	ErrStateUnavailable  = &BridgeError{Code: ErrorStateUnavailable}
	ErrChainNotSupported = &BridgeError{Code: ErrorChainNotSupported}
	ErrConnectionError   = &BridgeError{Code: ErrorConnectionError}
)
