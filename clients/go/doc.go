// Package tychobridge provides a Go client for the Tycho Bridge gRPC service (v1 API).
//
// The Tycho Bridge service provides zero-network-latency swap simulations against
// locally cached blockchain state. This client handles connection management,
// retries, and provides an idiomatic Go interface for all 13 gRPC methods.
//
// # Quick Start
//
// Create a client and perform a basic swap simulation:
//
//	client, err := tychobridge.NewClient("localhost:50051")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer client.Close()
//
//	resp, err := client.GetAmountOut(ctx, &tychobridge.GetAmountOutRequest{
//	    Chain:       tychobridge.ChainEthereum,
//	    ComponentID: "0x...",
//	    TokenIn:     "0x...",
//	    TokenOut:    "0x...",
//	    AmountIn:    tychobridge.NewBigInt("1000000000000000000"),
//	})
//
// # API Methods
//
// The client provides the following methods aligned with the Tycho SDK:
//
// Data Access:
//   - [Client.GetProtocolComponents] - Query protocol components (pools, markets)
//   - [Client.GetTokens] - Query tokens with quality filtering
//   - [Client.GetStatus] - Service health and per-chain statistics
//
// Simulation (ProtocolSim trait):
//   - [Client.GetAmountOut] - Swap simulation (get_amount_out)
//   - [Client.SpotPrice] - Marginal price query (spot_price)
//   - [Client.GetFee] - Protocol fee query (fee)
//   - [Client.GetLimits] - Trading limits query (get_limits)
//   - [Client.SwapToPrice] - Price-targeting trade (swap_to_price)
//   - [Client.QuerySupply] - Supply curve query (query_supply)
//
// Batch & Routing:
//   - [Client.BatchGetAmountOut] - Parallel batch simulation
//   - [Client.SimulateRoute] - Multi-hop simulation with state chaining
//
// Execution:
//   - [Client.EncodeSolution] - Generate transaction calldata
//
// Streaming:
//   - [Client.StreamUpdates] - Real-time state updates
//   - [Client.StreamUpdatesWithReconnect] - Auto-reconnecting stream
//
// # State Chaining
//
// Use the NewPoolState from simulation results to chain multiple simulations:
//
//	// First simulation
//	resp1, _ := client.GetAmountOut(ctx, req1)
//
//	// Second simulation uses state from first
//	req2.PoolStateOverride = resp1.NewPoolState
//	resp2, _ := client.GetAmountOut(ctx, req2)
//
// This enables accurate multi-hop routing without persisting state server-side.
//
// # Connection Options
//
// Configure the client with functional options:
//
//	client, err := tychobridge.NewClient("localhost:50051",
//	    tychobridge.WithDialTimeout(5*time.Second),
//	    tychobridge.WithRequestTimeout(10*time.Second),
//	    tychobridge.WithRetryPolicy(3, 100*time.Millisecond, 5*time.Second),
//	    tychobridge.WithKeepalive(30*time.Second, 10*time.Second),
//	)
//
// # Thread Safety
//
// All Client methods are safe for concurrent use from multiple goroutines.
// The client maintains a single gRPC connection with automatic keepalive.
package tychobridge
