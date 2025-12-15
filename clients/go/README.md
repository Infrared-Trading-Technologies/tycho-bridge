# Tycho Bridge Go Client

A Go client library for the Tycho Bridge gRPC service (v1 API). This client provides access to zero-network-latency swap simulations against locally cached blockchain state.

## Installation

```bash
go get github.com/branched-services/tycho-bridge-go
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"
    "time"

    tychobridge "github.com/branched-services/tycho-bridge-go"
)

func main() {
    // Create client with options
    client, err := tychobridge.NewClient("localhost:50051",
        tychobridge.WithDialTimeout(5*time.Second),
        tychobridge.WithRequestTimeout(10*time.Second),
    )
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    ctx := context.Background()

    // Check service status
    status, err := client.GetStatus(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Healthy: %v, Chains: %d\n", status.Healthy, len(status.Chains))

    // Query protocol components (pools)
    components, err := client.GetProtocolComponents(ctx, &tychobridge.GetProtocolComponentsRequest{
        Chain:           tychobridge.ChainEthereum,
        ProtocolSystems: []string{"uniswap_v2"},
        PageSize:        10,
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Found %d components\n", len(components.Components))

    // Simulate a swap (zero network calls)
    if len(components.Components) > 0 {
        comp := components.Components[0]
        sim, err := client.GetAmountOut(ctx, &tychobridge.GetAmountOutRequest{
            Chain:       tychobridge.ChainEthereum,
            ComponentID: comp.ID,
            TokenIn:     comp.Tokens[0],
            TokenOut:    comp.Tokens[1],
            AmountIn:    tychobridge.NewBigInt("1000000000000000000"), // 1 token
        })
        if err != nil {
            log.Fatal(err)
        }
        fmt.Printf("Amount out: %s\n", sim.Amount.Value)
    }
}
```

## Features

- **Full gRPC API Coverage**: All 13 TychoBridge v1 RPC methods implemented
- **Connection Management**: Automatic keepalive, configurable timeouts
- **Retry Logic**: Exponential backoff for transient failures
- **Streaming Support**: Real-time state updates with auto-reconnection
- **TLS Support**: Optional TLS/mTLS configuration
- **Thread-Safe**: Safe for concurrent use from multiple goroutines
- **State Chaining**: Chain simulations using `NewPoolState` for routing optimization

## API Reference

### Client Creation

```go
// Default options
client, err := tychobridge.NewClient("localhost:50051")

// With options
client, err := tychobridge.NewClient("localhost:50051",
    tychobridge.WithDialTimeout(5*time.Second),
    tychobridge.WithRequestTimeout(10*time.Second),
    tychobridge.WithRetryPolicy(3, 100*time.Millisecond, 5*time.Second),
    tychobridge.WithKeepalive(30*time.Second, 10*time.Second),
)
```

### Available Options

| Option                            | Description                             | Default              |
| --------------------------------- | --------------------------------------- | -------------------- |
| `WithDialTimeout(d)`              | Connection establishment timeout        | 10s                  |
| `WithRequestTimeout(d)`           | Individual request timeout              | 30s                  |
| `WithRetryPolicy(max, base, max)` | Retry configuration                     | 3 retries, 100ms-10s |
| `WithKeepalive(time, timeout)`    | Connection keepalive                    | 30s/10s              |
| `WithTLS(creds)`                  | Enable TLS with credentials             | Disabled             |
| `WithInsecureTLS()`               | TLS without verification (testing only) | Disabled             |
| `WithDialOptions(opts...)`        | Additional gRPC dial options            | None                 |

### Chains

The client uses a `Chain` enum for type-safe chain selection:

```go
tychobridge.ChainEthereum   // Ethereum Mainnet
tychobridge.ChainBase       // Base
tychobridge.ChainUnichain   // Unichain
```

### Methods

#### GetProtocolComponents

Query protocol components (pools, markets) from the local cache.

```go
resp, err := client.GetProtocolComponents(ctx, &tychobridge.GetProtocolComponentsRequest{
    Chain:           tychobridge.ChainEthereum,
    ProtocolSystems: []string{"uniswap_v2", "uniswap_v3"},
    TokenAddresses:  []string{"0x..."},       // Filter by tokens
    ComponentIDs:    []string{"0x..."},       // Filter by specific IDs
    MinTVL:          floatPtr(10000),         // Min TVL filter
    Page:            1,
    PageSize:        100,
})
// resp.Components, resp.TotalCount
```

#### GetTokens

Query tokens from the local cache.

```go
resp, err := client.GetTokens(ctx, &tychobridge.GetTokensRequest{
    Chain:        tychobridge.ChainEthereum,
    Addresses:    []string{"0x..."},
    SymbolPrefix: "UNI",
    MinQuality:   intPtr(50),  // Filter by quality score
    Page:         1,
    PageSize:     100,
})
// resp.Tokens[0].Address, .Symbol, .Decimals, .Quality
```

#### GetAmountOut

Execute a local swap simulation (zero network calls). Aligns with Tycho SDK's `ProtocolSim::get_amount_out()`.

```go
resp, err := client.GetAmountOut(ctx, &tychobridge.GetAmountOutRequest{
    Chain:             tychobridge.ChainEthereum,
    ComponentID:       "0x...",
    TokenIn:           "0x...",
    TokenOut:          "0x...",
    AmountIn:          tychobridge.NewBigInt("1000000000000000000"),
    PoolStateOverride: nil, // Optional: chain simulations
})
// resp.Amount, resp.Gas, resp.EffectivePrice, resp.PriceImpactBPS
// resp.NewPoolState - use for chained simulations
```

#### SpotPrice

Get the marginal price of a pool.

```go
resp, err := client.SpotPrice(ctx, &tychobridge.SpotPriceRequest{
    Chain:       tychobridge.ChainEthereum,
    ComponentID: "0x...",
    BaseToken:   "0x...",  // Token you're selling
    QuoteToken:  "0x...",  // Token you're buying
})
// resp.Price (float64), resp.PriceRational (Price)
```

#### GetFee

Get the protocol fee for a component.

```go
resp, err := client.GetFee(ctx, &tychobridge.GetFeeRequest{
    Chain:       tychobridge.ChainEthereum,
    ComponentID: "0x...",
})
// resp.Fee (0.003 = 0.3%), resp.FeeBPS (30), resp.IsDynamic
```

#### GetLimits

Get the maximum tradeable amounts between two tokens.

```go
resp, err := client.GetLimits(ctx, &tychobridge.GetLimitsRequest{
    Chain:       tychobridge.ChainEthereum,
    ComponentID: "0x...",
    SellToken:   "0x...",
    BuyToken:    "0x...",
})
// resp.MaxSellAmount, resp.MaxBuyAmount, resp.IsSoftLimit
```

#### SwapToPrice

Calculate the trade required to move pool price to a target.

```go
resp, err := client.SwapToPrice(ctx, &tychobridge.SwapToPriceRequest{
    Chain:       tychobridge.ChainEthereum,
    ComponentID: "0x...",
    TokenIn:     "0x...",
    TokenOut:    "0x...",
    TargetPrice: &tychobridge.Price{
        Numerator:   tychobridge.NewBigInt("1000"),
        Denominator: tychobridge.NewBigInt("1"),
    },
})
// resp.Trade.AmountIn, resp.Trade.AmountOut, resp.IsReachable
```

#### QuerySupply

Get the maximum supply a pool can provide at a price floor.

```go
resp, err := client.QuerySupply(ctx, &tychobridge.QuerySupplyRequest{
    Chain:       tychobridge.ChainEthereum,
    ComponentID: "0x...",
    TokenIn:     "0x...",
    TokenOut:    "0x...",
    MinPrice: &tychobridge.Price{
        Numerator:   tychobridge.NewBigInt("990"),
        Denominator: tychobridge.NewBigInt("1000"),
    },
})
// resp.Trade.AmountIn, resp.Trade.AmountOut
```

#### BatchGetAmountOut

Execute multiple simulations in parallel for routing optimization.

```go
resp, err := client.BatchGetAmountOut(ctx, &tychobridge.BatchGetAmountOutRequest{
    Requests: []*tychobridge.GetAmountOutRequest{
        {Chain: tychobridge.ChainEthereum, ComponentID: "0x...", ...},
        {Chain: tychobridge.ChainEthereum, ComponentID: "0x...", ...},
    },
    Parallel: true, // Execute in parallel (default)
})
// resp.Results[i].Success or resp.Results[i].Error
// resp.TotalExecutionTimeUs
```

#### SimulateRoute

Simulate a multi-hop swap path with state transitions.

```go
resp, err := client.SimulateRoute(ctx, &tychobridge.SimulateRouteRequest{
    Chain: tychobridge.ChainEthereum,
    Hops: []*tychobridge.RouteHop{
        {ComponentID: "pool1", TokenIn: "0xA", TokenOut: "0xB"},
        {ComponentID: "pool2", TokenIn: "0xB", TokenOut: "0xC"},
    },
    AmountIn: tychobridge.NewBigInt("1000000000000000000"),
})
// resp.AmountOut, resp.TotalGas, resp.TotalPriceImpactBPS
// resp.Hops[i].AmountIn, .AmountOut, .EffectivePrice, .PriceImpactBPS
// resp.FinalPoolStates - use for further simulations
```

#### EncodeSolution

Convert a Solution into calldata for the Tycho Router.

```go
resp, err := client.EncodeSolution(ctx, &tychobridge.EncodeSolutionRequest{
    Chain: tychobridge.ChainEthereum,
    Solution: &tychobridge.Solution{
        GivenToken:    "0x...",
        GivenAmount:   tychobridge.NewBigInt("1000000000000000000"),
        CheckedToken:  "0x...",
        Sender:        "0x...",
        Receiver:      "0x...",
        Swaps: []*tychobridge.Swap{
            {Component: comp, TokenIn: "0x...", TokenOut: "0x...", Split: 1.0},
        },
    },
    EncoderType:      tychobridge.EncoderTypeTychoRouter,
    UserTransferType: tychobridge.UserTransferTypeTransferFrom,
})
// resp.EncodedSwaps, resp.InteractingWith, resp.Selector
```

#### StreamUpdates

Subscribe to real-time state changes.

```go
stream, err := client.StreamUpdates(ctx, &tychobridge.StreamUpdatesRequest{
    Chain:                tychobridge.ChainEthereum,
    ProtocolSystems:      []string{"uniswap_v2"},
    ComponentIDs:         []string{"0x..."}, // Optional: specific components
    IncludeContractState: false,             // Include VM contract state
})
if err != nil {
    log.Fatal(err)
}
defer stream.Close()

for {
    select {
    case update := <-stream.Updates():
        fmt.Printf("Block %d: %d new, %d removed, %d modified\n",
            update.BlockNumber,
            len(update.NewComponents),
            len(update.RemovedComponentIDs),
            len(update.StateUpdates))
    case err := <-stream.Errors():
        log.Printf("Error: %v\n", err)
        return
    }
}
```

For automatic reconnection on errors:

```go
stream, err := client.StreamUpdatesWithReconnect(ctx, req)
```

#### GetStatus

Get service health and per-chain statistics.

```go
status, err := client.GetStatus(ctx)
// status.Healthy, status.Version, status.UptimeSeconds
// status.Chains[i].Chain, .Synced, .LatestBlock, .ComponentCount, .TokenCount
```

## Error Handling

The client defines several error types:

```go
var (
    ErrClientClosed     = errors.New("client is closed")
    ErrNilResponse      = errors.New("server returned nil response")
    ErrStreamClosed     = errors.New("stream is closed")
    ErrInvalidAddress   = errors.New("invalid address format")
    ErrInvalidAmount    = errors.New("invalid amount")
    ErrPoolNotFound     = errors.New("pool not found")
    ErrTokenNotFound    = errors.New("token not found")
    ErrSimulationFailed = errors.New("swap simulation failed")
)
```

For batch operations, individual errors are returned per-request:

```go
resp, _ := client.BatchGetAmountOut(ctx, req)
for i, result := range resp.Results {
    if result.Error != nil {
        fmt.Printf("Request %d failed: %s (code: %d)\n", 
            i, result.Error.Message, result.Error.Code)
    } else {
        fmt.Printf("Request %d: %s\n", i, result.Success.Amount.Value)
    }
}
```

All RPC errors are wrapped and include the original gRPC error for detailed diagnostics.

## Regenerating Proto Files

If you need to regenerate the protobuf/gRPC code:

```bash
# Install protoc plugins
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest

# Generate code (from project root)
protoc --go_out=clients/go/gen --go_opt=paths=source_relative \
       --go-grpc_out=clients/go/gen --go-grpc_opt=paths=source_relative \
       --experimental_allow_proto3_optional \
       --proto_path=. proto/tycho/bridge/v1/bridge.proto
```

## Migration from v0 API

If you're migrating from the previous API, here's a quick mapping:

| Old Method          | New Method              | Notes                         |
| ------------------- | ----------------------- | ----------------------------- |
| `SimulateSwap`      | `GetAmountOut`          | Aligns with ProtocolSim trait |
| `GetPools`          | `GetProtocolComponents` | Uses Tycho terminology        |
| `EncodeSwap`        | `EncodeSolution`        | Full Solution struct support  |
| `StreamPoolUpdates` | `StreamUpdates`         | Includes contract state       |
| `GetServiceStatus`  | `GetStatus`             | Per-chain status              |

New methods added in v1:
- `SpotPrice` - Marginal price query
- `GetFee` - Fee query
- `GetLimits` - Trading limit query
- `SwapToPrice` - Price-targeting trade
- `QuerySupply` - Supply curve query
- `BatchGetAmountOut` - Parallel simulation
- `SimulateRoute` - Multi-hop simulation

## Examples

See the [examples](./examples/) directory for complete working examples:

- [basic](./examples/basic/) - Basic usage: status, components, tokens, simulation
- [streaming](./examples/streaming/) - Real-time state updates with reconnection

## License

This client is part of the Tycho Bridge project.
