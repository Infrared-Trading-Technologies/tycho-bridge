package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	tychobridge "github.com/branched-services/tycho-bridge-go"
)

func main() {
	addr := os.Getenv("TYCHO_BRIDGE_ADDR")
	if addr == "" {
		addr = "localhost:50051"
	}

	client, err := tychobridge.NewClient(addr,
		tychobridge.WithDialTimeout(5*time.Second),
		tychobridge.WithRequestTimeout(10*time.Second),
		tychobridge.WithRetryPolicy(3, 100*time.Millisecond, 5*time.Second),
	)
	if err != nil {
		log.Fatalf("Failed to create client: %v", err)
	}
	defer client.Close()

	ctx := context.Background()

	// Get service status
	status, err := client.GetStatus(ctx)
	if err != nil {
		log.Fatalf("Failed to get service status: %v", err)
	}

	fmt.Printf("Service Status:\n")
	fmt.Printf("  Healthy: %v\n", status.Healthy)
	fmt.Printf("  Version: %s\n", status.Version)
	fmt.Printf("  Uptime: %d seconds\n", status.UptimeSeconds)
	fmt.Printf("  Chains:\n")
	for _, chain := range status.Chains {
		fmt.Printf("    Chain %d: synced=%v, block=%d, components=%d, tokens=%d\n",
			chain.Chain, chain.Synced, chain.LatestBlock, chain.ComponentCount, chain.TokenCount)
	}
	fmt.Println()

	// Get protocol components (pools)
	componentsResp, err := client.GetProtocolComponents(ctx, &tychobridge.GetProtocolComponentsRequest{
		Chain:           tychobridge.ChainEthereum,
		ProtocolSystems: []string{"uniswap_v2"},
		PageSize:        5,
	})
	if err != nil {
		log.Fatalf("Failed to get components: %v", err)
	}

	fmt.Printf("Found %d Uniswap V2 components on Ethereum:\n", componentsResp.TotalCount)
	for _, comp := range componentsResp.Components {
		idDisplay := comp.ID
		if len(idDisplay) > 16 {
			idDisplay = comp.ID[:16] + "..."
		}
		fmt.Printf("  %s: tokens=%v, protocol=%s\n", idDisplay, comp.Tokens, comp.ProtocolSystem)
	}
	fmt.Println()

	// Get tokens
	tokensResp, err := client.GetTokens(ctx, &tychobridge.GetTokensRequest{
		Chain:    tychobridge.ChainEthereum,
		PageSize: 5,
	})
	if err != nil {
		log.Fatalf("Failed to get tokens: %v", err)
	}

	fmt.Printf("Found %d tokens:\n", tokensResp.TotalCount)
	for _, token := range tokensResp.Tokens {
		addrDisplay := token.Address
		if len(addrDisplay) > 10 {
			addrDisplay = token.Address[:10] + "..."
		}
		fmt.Printf("  %s (%s): decimals=%d, quality=%d\n",
			token.Symbol, addrDisplay, token.Decimals, token.Quality)
	}
	fmt.Println()

	// Simulate a swap using GetAmountOut
	if len(componentsResp.Components) > 0 && len(componentsResp.Components[0].Tokens) >= 2 {
		comp := componentsResp.Components[0]
		tokenIn := comp.Tokens[0]
		tokenOut := comp.Tokens[1]

		// Run multiple iterations to see warmed-up performance
		const iterations = 100
		var totalRoundTrip, totalExecution, totalSimulation int64
		var firstRoundTrip int64
		var simResp *tychobridge.GetAmountOutResponse
		var err error

		for i := 0; i < iterations; i++ {
			startTime := time.Now()
			simResp, err = client.GetAmountOut(ctx, &tychobridge.GetAmountOutRequest{
				Chain:       tychobridge.ChainEthereum,
				ComponentID: comp.ID,
				TokenIn:     tokenIn,
				TokenOut:    tokenOut,
				AmountIn:    tychobridge.NewBigInt("1000000000000000000"), // 1 token
			})
			roundTripTime := time.Since(startTime).Microseconds()

			if err != nil {
				log.Printf("Swap simulation failed: %v", err)
				break
			}

			if i == 0 {
				firstRoundTrip = roundTripTime
			}
			totalRoundTrip += roundTripTime
			totalExecution += int64(simResp.ExecutionTimeUs)
			totalSimulation += int64(simResp.SimulationTimeUs)
		}

		if err == nil {
			idDisplay := comp.ID
			if len(idDisplay) > 16 {
				idDisplay = comp.ID[:16] + "..."
			}
			fmt.Printf("Swap Simulation (GetAmountOut) - %d iterations:\n", iterations)
			fmt.Printf("  Component: %s\n", idDisplay)
			fmt.Printf("  %s -> %s\n", tokenIn, tokenOut)
			fmt.Printf("  Amount Out: %s\n", simResp.Amount.Value)
			fmt.Printf("  Gas Estimate: %s\n", simResp.Gas.Value)
			fmt.Printf("  Price Impact: %.2f bps\n", simResp.PriceImpactBPS)
			fmt.Printf("  Effective Price: %.6f\n", simResp.EffectivePrice)

			avgRoundTrip := totalRoundTrip / iterations
			avgExecution := totalExecution / iterations
			avgSimulation := totalSimulation / iterations
			avgOverhead := avgRoundTrip - avgExecution

			fmt.Printf("\n  Timing Breakdown (avg of %d iterations):\n", iterations)
			fmt.Printf("    First call round-trip:  %d µs\n", firstRoundTrip)
			fmt.Printf("    Avg round-trip:         %d µs\n", avgRoundTrip)
			fmt.Printf("    Avg server execution:   %d µs\n", avgExecution)
			fmt.Printf("    Avg pure AMM math:      %d µs\n", avgSimulation)
			fmt.Printf("    Avg gRPC overhead:      %d µs\n", avgOverhead)
		}
	}
}
