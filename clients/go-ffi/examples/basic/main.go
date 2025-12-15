// Package main demonstrates basic usage of the Tycho Bridge FFI client.
package main

import (
	"fmt"
	"log"
	"os"
	"strings"
	"time"

	ffi "github.com/propeller-heads/tycho-bridge-ffi"
)

func main() {
	// Get configuration from environment
	apiKey := os.Getenv("TYCHO_API_KEY")
	if apiKey == "" {
		log.Println("Warning: TYCHO_API_KEY not set, using empty key")
	}

	tychoURL := os.Getenv("TYCHO_URL")
	if tychoURL == "" {
		tychoURL = "tycho-beta.propellerheads.xyz"
	}

	// Print library version
	fmt.Printf("Tycho Bridge FFI Version: %s\n", ffi.Version())

	// Create configuration
	config := ffi.Config{
		TychoURL:    tychoURL,
		TychoAPIKey: apiKey,
		ChainIDs:    []uint64{1}, // Ethereum mainnet
	}

	fmt.Printf("Connecting to %s...\n", config.TychoURL)

	// Initialize bridge
	bridge, err := ffi.NewBridge(config)
	if err != nil {
		log.Fatalf("Failed to initialize bridge: %v", err)
	}
	defer bridge.Close()

	fmt.Println("Bridge initialized successfully")

	// Start syncing
	if err := bridge.Start(); err != nil {
		log.Fatalf("Failed to start bridge: %v", err)
	}

	fmt.Println("Syncing started, waiting for initial sync...")

	// Wait for initial sync
	for i := 0; i < 30; i++ {
		time.Sleep(1 * time.Second)
		count := bridge.ComponentCount()
		fmt.Printf("  Components synced: %d\n", count)
		if count > 0 {
			break
		}
	}

	// Check health
	healthy := bridge.IsHealthy()
	fmt.Printf("Bridge healthy: %v\n", healthy)

	// Get component count
	count := bridge.ComponentCount()
	fmt.Printf("Total components: %d\n", count)

	// Get uptime
	uptime := bridge.Uptime()
	fmt.Printf("Uptime: %s\n", uptime)

	// Example addresses (WETH and USDC on Ethereum)
	weth := "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
	usdc := "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"

	// Get components and filter for WETH/USDC pools
	components, err := bridge.GetComponents(ffi.GetComponentsRequest{
		ChainID: 1,
		Limit:   500, // Get more to find WETH/USDC pools
	})
	if err != nil {
		log.Printf("Failed to get components: %v", err)
	}

	// Find pools that contain both WETH and USDC (case-insensitive)
	wethLower := strings.ToLower(weth)
	usdcLower := strings.ToLower(usdc)
	var wethUsdcPools []ffi.Component
	if components != nil {
		for _, c := range components.Components {
			hasWeth := false
			hasUsdc := false
			for _, token := range c.Tokens {
				tokenLower := strings.ToLower(token)
				if tokenLower == wethLower {
					hasWeth = true
				}
				if tokenLower == usdcLower {
					hasUsdc = true
				}
			}
			if hasWeth && hasUsdc {
				wethUsdcPools = append(wethUsdcPools, c)
			}
		}
	}

	fmt.Printf("\nFound %d WETH/USDC pools out of %d components:\n", len(wethUsdcPools), len(components.Components))
	for i, c := range wethUsdcPools {
		if i >= 5 {
			fmt.Printf("  ... and %d more\n", len(wethUsdcPools)-5)
			break
		}
		fmt.Printf("  %d. %s (%s)\n", i+1, c.ID[:20]+"...", c.ProtocolSystem)
	}

	// Fallback: if no WETH/USDC pools, find any pool with WETH
	var poolToUse ffi.Component
	var tokenOut string
	if len(wethUsdcPools) > 0 {
		poolToUse = wethUsdcPools[0]
		tokenOut = usdc
	} else {
		// Find any pool with WETH
		for _, c := range components.Components {
			hasWeth := false
			for _, token := range c.Tokens {
				if strings.ToLower(token) == wethLower {
					hasWeth = true
				} else {
					tokenOut = token
				}
			}
			if hasWeth && tokenOut != "" {
				poolToUse = c
				fmt.Printf("\nNo WETH/USDC pool found, using WETH/%s pool instead\n", tokenOut[:10]+"...")
				break
			}
		}
	}

	// If we have a pool to use, try simulations
	if poolToUse.ID != "" {
		fmt.Printf("\nUsing pool: %s (%s)\n", poolToUse.ID, poolToUse.ProtocolSystem)

		// Try to get spot price
		spotResp, err := bridge.SpotPrice(ffi.SpotPriceRequest{
			ChainID:     1,
			ComponentID: poolToUse.ID,
			BaseToken:   weth,
			QuoteToken:  tokenOut,
		})
		if err != nil {
			log.Printf("Spot price query failed: %v", err)
		} else {
			fmt.Printf("Spot price: %.6f\n", spotResp.PriceFloat)
		}

		// Try to simulate a swap
		swapResp, err := bridge.GetAmountOut(ffi.GetAmountOutRequest{
			ChainID:     1,
			ComponentID: poolToUse.ID,
			TokenIn:     weth,
			TokenOut:    tokenOut,
			AmountIn:    ffi.NewBigInt("1000000000000000000"), // 1 WETH (18 decimals)
		})
		if err != nil {
			log.Printf("Swap simulation failed: %v", err)
		} else {
			fmt.Printf("Swap 1 WETH -> %s output tokens\n", swapResp.AmountOut.Value)
			fmt.Printf("Gas cost: %s\n", swapResp.GasCost.Value)
		}
	} else {
		fmt.Println("\nNo pools found with WETH")
	}

	// Get tokens
	tokens, err := bridge.GetTokens(ffi.GetTokensRequest{
		ChainID: 1,
		Limit:   5,
	})
	if err != nil {
		log.Printf("Failed to get tokens: %v", err)
	} else {
		fmt.Printf("\nFirst %d tokens:\n", len(tokens.Tokens))
		for i, t := range tokens.Tokens {
			fmt.Printf("  %d. %s (%s) - %d decimals\n", i+1, t.Symbol, t.Address[:10]+"...", t.Decimals)
		}
	}

	// Get status
	status, err := bridge.GetStatus(ffi.GetStatusRequest{
		ChainID: 1,
	})
	if err != nil {
		log.Printf("Failed to get status: %v", err)
	} else {
		fmt.Printf("\nIndexer Status:\n")
		fmt.Printf("  Sync status: %s\n", status.Status.SyncStatus)
		fmt.Printf("  Latest block: %d\n", status.Status.LatestBlock)
		fmt.Printf("  Components: %d\n", status.Status.ComponentCount)
	}

	// Health check
	health, err := bridge.HealthCheck()
	if err != nil {
		log.Printf("Health check failed: %v", err)
	} else {
		fmt.Printf("\nHealth Check:\n")
		fmt.Printf("  Healthy: %v\n", health.Healthy)
		if health.Message != "" {
			fmt.Printf("  Message: %s\n", health.Message)
		}
	}

	fmt.Println("\nDone!")
}
