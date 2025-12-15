# Tycho Bridge Go FFI Client

High-performance Go client for tycho-bridge using FFI (Foreign Function Interface).

This client calls the Rust `libtycho_bridge` library directly via CGo, providing ~1000x lower latency compared to the gRPC client for co-located deployments.

## Features

- **Direct FFI calls** - No network overhead for local deployments
- **Thread-safe** - Safe for concurrent use from multiple goroutines
- **Automatic memory management** - Go garbage collector handles cleanup
- **Full API coverage** - All simulation, indexing, and status functions
- **Compatible types** - Same `BigInt`, `Price`, and `Token` types as the gRPC client

## Requirements

- Go 1.22 or later
- Rust toolchain (for building the FFI library)
- CGO enabled (`CGO_ENABLED=1`)

## Building

First, build the Rust FFI library:

```bash
cd tycho-bridge
cargo build -p tycho-ffi --release
```

Then build the Go package:

```bash
cd clients/go-ffi
make build
```

Or use the Makefile to build everything:

```bash
make all
```

## Quick Start

```go
package main

import (
    "log"

    ffi "github.com/propeller-heads/tycho-bridge-ffi"
)

func main() {
    // Create configuration
    config := ffi.Config{
        TychoURL:    "tycho-beta.propellerheads.xyz",
        TychoAPIKey: "your-api-key",
        ChainIDs:    []uint64{1, 8453}, // Ethereum and Base
    }

    // Initialize bridge
    bridge, err := ffi.NewBridge(config)
    if err != nil {
        log.Fatal(err)
    }
    defer bridge.Close()

    // Start syncing
    if err := bridge.Start(); err != nil {
        log.Fatal(err)
    }

    // Wait for sync (in production, use proper sync status checking)
    // time.Sleep(5 * time.Second)

    // Query spot price
    resp, err := bridge.SpotPrice(ffi.SpotPriceRequest{
        ChainID:     1,
        ComponentID: "0xpool...",
        BaseToken:   "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
        QuoteToken:  "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
    })
    if err != nil {
        log.Fatal(err)
    }

    log.Printf("Spot price: %v", resp.PriceFloat)

    // Simulate a swap
    swapResp, err := bridge.GetAmountOut(ffi.GetAmountOutRequest{
        ChainID:     1,
        ComponentID: "0xpool...",
        TokenIn:     "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        TokenOut:    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        AmountIn:    ffi.NewBigInt("1000000000000000000"), // 1 WETH
    })
    if err != nil {
        log.Fatal(err)
    }

    log.Printf("Amount out: %s", swapResp.AmountOut.Value)
}
```

## API Reference

### Bridge Lifecycle

```go
// Create new bridge
bridge, err := ffi.NewBridge(config)

// Start syncing
err := bridge.Start()

// Stop syncing
err := bridge.Stop()

// Close and release resources
bridge.Close()

// Check health
healthy := bridge.IsHealthy()

// Get component count
count := bridge.ComponentCount()

// Get uptime
uptime := bridge.Uptime()

// Get library version
version := ffi.Version()
```

### Simulation Methods

```go
// Spot price query
resp, err := bridge.SpotPrice(ffi.SpotPriceRequest{...})

// Single swap simulation
resp, err := bridge.GetAmountOut(ffi.GetAmountOutRequest{...})

// Get pool fee
resp, err := bridge.GetFee(ffi.GetFeeRequest{...})

// Get trading limits
resp, err := bridge.GetLimits(ffi.GetLimitsRequest{...})

// Calculate swap to reach target price
resp, err := bridge.SwapToPrice(ffi.SwapToPriceRequest{...})

// Multi-hop route simulation
resp, err := bridge.SimulateRoute(ffi.SimulateRouteRequest{...})

// Batch simulations (parallel)
resp, err := bridge.BatchGetAmountOut(ffi.BatchGetAmountOutRequest{...})
```

### Indexing Methods

```go
// Get tokens
resp, err := bridge.GetTokens(ffi.GetTokensRequest{...})

// Get components
resp, err := bridge.GetComponents(ffi.GetComponentsRequest{...})
```

### Status Methods

```go
// Get indexer status
resp, err := bridge.GetStatus(ffi.GetStatusRequest{...})

// Health check
status, err := bridge.HealthCheck()
```

## Error Handling

All errors are returned as `*BridgeError`:

```go
resp, err := bridge.SpotPrice(req)
if err != nil {
    var bridgeErr *ffi.BridgeError
    if errors.As(err, &bridgeErr) {
        switch bridgeErr.Code {
        case ffi.ErrorComponentNotFound:
            log.Printf("Component not found: %s", bridgeErr.Message)
        case ffi.ErrorSimulationFailed:
            log.Printf("Simulation failed: %s", bridgeErr.Message)
        default:
            log.Printf("Error %s: %s", bridgeErr.Code, bridgeErr.Message)
        }
    }
}
```

## Thread Safety

The `Bridge` is safe for concurrent use from multiple goroutines. Internally, it uses a read-write mutex to protect access to the underlying FFI handle.

```go
// Safe to use from multiple goroutines
var wg sync.WaitGroup
for i := 0; i < 10; i++ {
    wg.Add(1)
    go func() {
        defer wg.Done()
        resp, err := bridge.SpotPrice(req)
        // ...
    }()
}
wg.Wait()
```

## Performance

The FFI client provides significantly lower latency than gRPC for co-located deployments:

| Operation               | gRPC Latency | FFI Latency | Speedup |
| ----------------------- | ------------ | ----------- | ------- |
| SpotPrice               | ~100µs       | ~100ns      | 1000x   |
| GetAmountOut            | ~500µs       | ~1µs        | 500x    |
| BatchGetAmountOut (100) | ~5ms         | ~50µs       | 100x    |

*Benchmarks measured on local deployment. Network latency dominates gRPC performance.*

## Comparison with gRPC Client

| Feature           | gRPC Client       | FFI Client             |
| ----------------- | ----------------- | ---------------------- |
| Network required  | Yes               | No                     |
| Latency           | Higher            | Lower                  |
| Language bindings | Any gRPC language | CGo only               |
| Deployment        | Separate service  | Same process           |
| Streaming         | Yes               | No (callbacks planned) |

Use FFI when:
- Tycho Bridge runs in the same process/container
- Ultra-low latency is critical
- You're using Go

Use gRPC when:
- Remote deployment required
- Using languages without CGo support
- Need streaming updates

## Testing

```bash
# Run unit tests (no FFI required)
make test-short

# Run all tests (requires Rust lib)
make test

# Run with race detector
make test-race

# Run benchmarks
make bench
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
