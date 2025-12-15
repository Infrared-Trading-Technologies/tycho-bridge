# Tycho Bridge

High-performance DeFi simulation library wrapping the [Propeller Heads Tycho SDK](https://github.com/propeller-heads/tycho-simulation). Provides **zero-I/O local swap simulations** with multi-language support via FFI and gRPC.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Your Application                         │
├──────────────────────────┬──────────────────────────────────────┤
│     Go FFI Client        │         Go gRPC Client               │
│   (clients/go-ffi)       │         (clients/go)                 │
├──────────────────────────┼──────────────────────────────────────┤
│      C FFI Layer         │         gRPC Server                  │
│    (tycho-ffi)           │       (tycho-grpc)                   │
├──────────────────────────┴──────────────────────────────────────┤
│                     Core Business Logic                          │
│                       (tycho-core)                               │
├─────────────────────────────────────────────────────────────────┤
│              Tycho SDK (tycho-simulation, tycho-execution)       │
└─────────────────────────────────────────────────────────────────┘
```

### Crates

| Crate        | Description                                                         |
| ------------ | ------------------------------------------------------------------- |
| `tycho-core` | Transport-agnostic business logic, state cache, simulation handlers |
| `tycho-ffi`  | C FFI bindings for embedding in any language (Go, Python, etc.)     |
| `tycho-grpc` | gRPC server for network-based access                                |

### Key Features

- **Zero-Network Simulation** - `GetAmountOut` executes against local memory (5-3000μs)
- **Multi-Chain** - Ethereum, Base, Unichain, Arbitrum
- **Dual Interface** - FFI for embedded use, gRPC for service architecture
- **Official Primitives** - Built on `tycho-simulation` and `tycho-execution`

## Quick Start

```bash
# Clone and setup
git clone https://github.com/propeller-heads/tycho-bridge.git
cd tycho-bridge
./init.sh

# Build release (generates shared library + C headers)
cargo build --release
```

### Prerequisites

- Rust 1.70+
- Go 1.22+ (for Go clients)
- protoc (for gRPC)

## Usage

### FFI (Recommended for Go)

The FFI interface provides direct, low-latency access:

```go
import "github.com/propeller-heads/tycho-bridge-ffi"

bridge, _ := tycho.NewBridge(tycho.Config{
    TychoURL:    "wss://tycho-beta.propellerheads.xyz",
    TychoAPIKey: os.Getenv("TYCHO_API_KEY"),
    ChainIDs:    []uint64{1, 8453},
})
defer bridge.Close()

result, _ := bridge.GetAmountOut(tycho.GetAmountOutRequest{
    ChainID:     1,
    ComponentID: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
    TokenIn:     "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    TokenOut:    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    AmountIn:    "1000000000000000000",
})
```

### gRPC

For service-oriented architectures:

```bash
# Start the gRPC server
cargo run --release

# Connect from Go
import "github.com/propeller-heads/tycho-bridge-go"

client, _ := tycho.NewClient("localhost:50051")
```

## Configuration

Environment variables (see `.env.example`):

```bash
TYCHO_URL="wss://tycho-beta.propellerheads.xyz"
TYCHO_API_KEY="your-api-key"
CHAIN_IDS="1,8453,130,42161"
GRPC_PORT=50051
METRICS_PORT=9000
```

## API Overview

### Simulation

| Method              | Description                   |
| ------------------- | ----------------------------- |
| `GetAmountOut`      | Simulate swap output          |
| `SpotPrice`         | Get marginal price            |
| `GetLimits`         | Get tradeable limits          |
| `BatchGetAmountOut` | Parallel batch simulation     |
| `SimulateRoute`     | Multi-hop with state chaining |

### Data Access

| Method                  | Description                  |
| ----------------------- | ---------------------------- |
| `GetTokens`             | Query tokens with filtering  |
| `GetProtocolComponents` | Query pools/markets          |
| `StreamUpdates`         | Real-time state subscription |

### Execution

| Method           | Description                   |
| ---------------- | ----------------------------- |
| `EncodeSolution` | Generate transaction calldata |

## Project Structure

```
tycho-bridge/
├── crates/
│   ├── tycho-core/       # Core library
│   ├── tycho-ffi/        # C FFI bindings
│   └── tycho-grpc/       # gRPC server
├── clients/
│   ├── go-ffi/           # Go FFI client
│   └── go/               # Go gRPC client
├── include/              # Generated C headers
├── proto/                # Protocol definitions
└── init.sh               # Development setup
```

## Development

```bash
# Run tests (231 tests)
cargo test --workspace

# Generate docs
cargo doc --open

# Build C header
cargo build -p tycho-ffi  # outputs to include/tycho_bridge.h
```

## License

MIT License - see [LICENSE](LICENSE)
