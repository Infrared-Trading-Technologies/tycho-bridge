// Package tychoffi provides a high-performance Go client for tycho-bridge using FFI.
//
// This client uses CGo to call the Rust tycho-bridge library directly, providing
// ~1000x lower latency than the gRPC client for co-located deployments.
//
// # Quick Start
//
//	import ffi "github.com/propeller-heads/tycho-bridge-ffi"
//
//	// Create configuration
//	config := ffi.Config{
//	    TychoURL:    "tycho-beta.propellerheads.xyz",
//	    TychoAPIKey: "your-api-key",
//	    ChainIDs:    []uint64{1, 8453},
//	}
//
//	// Initialize bridge
//	bridge, err := ffi.NewBridge(config)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer bridge.Close()
//
//	// Start syncing
//	if err := bridge.Start(); err != nil {
//	    log.Fatal(err)
//	}
//
//	// Query spot price
//	price, err := bridge.SpotPrice(ffi.SpotPriceRequest{
//	    ChainID:     1,
//	    ComponentID: "0x...",
//	    BaseToken:   "0x...",
//	    QuoteToken:  "0x...",
//	})
//
// # Building
//
// The FFI client requires the Rust library to be built first:
//
//	cd tycho-bridge
//	cargo build -p tycho-ffi --release
//
// Then build the Go client:
//
//	cd clients/go-ffi
//	CGO_ENABLED=1 go build ./...
//
// # Thread Safety
//
// The Bridge is safe for concurrent use from multiple goroutines.
// All methods are thread-safe.
//
// # Memory Management
//
// The Bridge handles all memory management automatically. Results from
// FFI calls are copied into Go-managed memory before being returned.
//
// # Error Handling
//
// Errors are returned as Go error types. Detailed error codes are available
// via type assertions to *BridgeError.
package tychoffi
