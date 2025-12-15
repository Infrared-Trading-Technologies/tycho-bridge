package tychoffi

/*
#cgo CFLAGS: -I${SRCDIR}/../../include
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -ltycho_bridge -ldl -lm -lpthread
#include "tycho_bridge.h"
#include <stdlib.h>
#include <string.h>
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"
)

// Config holds the configuration for initializing the bridge.
type Config struct {
	// TychoURL is the Tycho indexer URL (without protocol prefix).
	// Example: "tycho-beta.propellerheads.xyz"
	TychoURL string `json:"tycho_url"`

	// TychoAPIKey is the API key for authentication.
	TychoAPIKey string `json:"tycho_api_key"`

	// ChainIDs is the list of chain IDs to sync.
	// Example: []uint64{1, 8453} for Ethereum and Base.
	ChainIDs []uint64 `json:"chain_ids"`

	// AuthenticationDisabled disables authentication (for local testing).
	AuthenticationDisabled bool `json:"authentication_disabled,omitempty"`

	// ProtocolsFilename is an optional path to a protocols filter file.
	ProtocolsFilename string `json:"protocols_filename,omitempty"`
}

// MarshalJSON implements custom JSON marshaling for Config.
func (c Config) MarshalJSON() ([]byte, error) {
	type Alias Config

	// Convert chain IDs to comma-separated string
	chainIDStr := ""
	for i, id := range c.ChainIDs {
		if i > 0 {
			chainIDStr += ","
		}
		chainIDStr += fmt.Sprintf("%d", id)
	}

	return json.Marshal(&struct {
		TychoURL               string `json:"tycho_url"`
		TychoAPIKey            string `json:"tycho_api_key"`
		ChainIDs               string `json:"chain_ids"`
		AuthenticationDisabled bool   `json:"authentication_disabled,omitempty"`
		ProtocolsFilename      string `json:"protocols_filename,omitempty"`
	}{
		TychoURL:               c.TychoURL,
		TychoAPIKey:            c.TychoAPIKey,
		ChainIDs:               chainIDStr,
		AuthenticationDisabled: c.AuthenticationDisabled,
		ProtocolsFilename:      c.ProtocolsFilename,
	})
}

// Bridge wraps the Rust tycho-bridge FFI.
// It is safe for concurrent use from multiple goroutines.
type Bridge struct {
	handle  *C.FfiBridge
	mu      sync.RWMutex
	closed  atomic.Bool
	started atomic.Bool
}

// NewBridge creates a new bridge instance from the configuration.
//
// The bridge must be started with Start() before making queries.
// The bridge must be closed with Close() when done.
//
// Example:
//
//	bridge, err := NewBridge(Config{
//	    TychoURL:    "tycho-beta.propellerheads.xyz",
//	    TychoAPIKey: "your-api-key",
//	    ChainIDs:    []uint64{1, 8453},
//	})
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer bridge.Close()
func NewBridge(config Config) (*Bridge, error) {
	configJSON, err := json.Marshal(config)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal config: %w", err)
	}

	cConfig := C.CString(string(configJSON))
	defer C.free(unsafe.Pointer(cConfig))

	handle := C.tycho_bridge_init(cConfig)
	if handle == nil {
		// Get error from last error
		errCode := C.tycho_last_error_code()
		errMsg := C.GoString(C.tycho_last_error_message())
		return nil, NewBridgeError(int32(errCode), errMsg)
	}

	b := &Bridge{handle: handle}
	runtime.SetFinalizer(b, (*Bridge).Close)
	return b, nil
}

// Close releases all resources associated with the bridge.
// After Close, the bridge cannot be used.
func (b *Bridge) Close() {
	if b == nil {
		return
	}

	// Use compare-and-swap to ensure we only close once
	if !b.closed.CompareAndSwap(false, true) {
		return
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	if b.handle != nil {
		C.tycho_bridge_free(b.handle)
		b.handle = nil
	}

	// Clear the finalizer since we've cleaned up
	runtime.SetFinalizer(b, nil)
}

// ensureOpen returns an error if the bridge is closed.
func (b *Bridge) ensureOpen() error {
	if b == nil {
		return ErrNilBridge
	}
	if b.closed.Load() {
		return ErrBridgeClosed
	}
	return nil
}

// Start begins the background synchronization task.
// This starts syncing component state from the Tycho indexer.
func (b *Bridge) Start() error {
	if err := b.ensureOpen(); err != nil {
		return err
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	result := C.tycho_bridge_start(b.handle)
	defer C.tycho_free_result(result)

	if !result.success {
		return NewBridgeError(int32(result.error_code), C.GoString(result.error_message))
	}

	b.started.Store(true)
	return nil
}

// Stop stops the background synchronization task.
func (b *Bridge) Stop() error {
	if err := b.ensureOpen(); err != nil {
		return err
	}

	b.mu.Lock()
	defer b.mu.Unlock()

	result := C.tycho_bridge_stop(b.handle)
	defer C.tycho_free_result(result)

	if !result.success {
		return NewBridgeError(int32(result.error_code), C.GoString(result.error_message))
	}

	b.started.Store(false)
	return nil
}

// IsStarted returns true if the bridge has been started.
func (b *Bridge) IsStarted() bool {
	return b != nil && b.started.Load()
}

// IsHealthy returns true if the bridge is healthy.
func (b *Bridge) IsHealthy() bool {
	if b == nil || b.closed.Load() {
		return false
	}
	b.mu.RLock()
	defer b.mu.RUnlock()
	return bool(C.tycho_bridge_is_healthy(b.handle))
}

// ComponentCount returns the number of components currently cached.
func (b *Bridge) ComponentCount() uint64 {
	if b == nil || b.closed.Load() {
		return 0
	}
	b.mu.RLock()
	defer b.mu.RUnlock()
	return uint64(C.tycho_bridge_component_count(b.handle))
}

// Uptime returns the bridge uptime.
func (b *Bridge) Uptime() time.Duration {
	if b == nil || b.closed.Load() {
		return 0
	}
	b.mu.RLock()
	defer b.mu.RUnlock()
	seconds := uint64(C.tycho_bridge_uptime(b.handle))
	return time.Duration(seconds) * time.Second
}

// Version returns the library version string.
func Version() string {
	return C.GoString(C.tycho_version())
}

// parseResult parses an FfiResult and returns the data or an error.
func parseResult(result C.FfiResult) ([]byte, error) {
	if !result.success {
		return nil, NewBridgeError(int32(result.error_code), C.GoString(result.error_message))
	}

	if result.data == nil || result.data_len == 0 {
		return nil, nil
	}

	// Copy data to Go-managed memory
	data := C.GoBytes(unsafe.Pointer(result.data), C.int(result.data_len))
	return data, nil
}
