// Package main demonstrates batch simulation using the Tycho Bridge FFI client.
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
	tychoURL := os.Getenv("TYCHO_URL")
	if tychoURL == "" {
		tychoURL = "tycho-beta.propellerheads.xyz"
	}

	// Create configuration
	config := ffi.Config{
		TychoURL:    tychoURL,
		TychoAPIKey: apiKey,
		ChainIDs:    []uint64{1}, // Ethereum mainnet
	}

	// Initialize bridge
	bridge, err := ffi.NewBridge(config)
	if err != nil {
		log.Fatalf("Failed to initialize bridge: %v", err)
	}
	defer bridge.Close()

	// Start syncing
	if err := bridge.Start(); err != nil {
		log.Fatalf("Failed to start bridge: %v", err)
	}

	// Wait for initial sync
	fmt.Println("Waiting for sync...")
	for i := 0; i < 30; i++ {
		time.Sleep(1 * time.Second)
		if bridge.ComponentCount() > 10 {
			break
		}
	}

	fmt.Printf("Synced %d components\n", bridge.ComponentCount())

	// Get components (note: MinTVL filter not yet implemented)
	components, err := bridge.GetComponents(ffi.GetComponentsRequest{
		ChainID: 1,
		Limit:   500, // Get more to find WETH/USDC pools
	})
	if err != nil {
		log.Fatalf("Failed to get components: %v", err)
	}

	if len(components.Components) == 0 {
		log.Fatal("No components available")
	}

	// Token addresses (case-insensitive matching)
	weth := strings.ToLower("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
	usdc := strings.ToLower("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")

	// Build batch request using available components
	var requests []ffi.GetAmountOutRequest
	for _, comp := range components.Components {
		// Check if component has WETH and USDC
		hasWeth := false
		hasUsdc := false
		for _, token := range comp.Tokens {
			tokenLower := strings.ToLower(token)
			if tokenLower == weth {
				hasWeth = true
			}
			if tokenLower == usdc {
				hasUsdc = true
			}
		}

		if hasWeth && hasUsdc {
			requests = append(requests, ffi.GetAmountOutRequest{
				ChainID:     1,
				ComponentID: comp.ID,
				TokenIn:     "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH (original case)
				TokenOut:    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC (original case)
				AmountIn:    ffi.NewBigInt("1000000000000000000"),         // 1 WETH
			})
		}

		if len(requests) >= 20 {
			break
		}
	}

	if len(requests) == 0 {
		fmt.Printf("No WETH/USDC pools found in %d components\n", len(components.Components))
		fmt.Println("Falling back to simulating first available pools with WETH...")

		// Find pools with just WETH
		for _, comp := range components.Components {
			hasWeth := false
			var otherToken string
			for _, token := range comp.Tokens {
				tokenLower := strings.ToLower(token)
				if tokenLower == weth {
					hasWeth = true
				} else {
					otherToken = token
				}
			}

			if hasWeth && otherToken != "" {
				requests = append(requests, ffi.GetAmountOutRequest{
					ChainID:     1,
					ComponentID: comp.ID,
					TokenIn:     "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
					TokenOut:    otherToken,
					AmountIn:    ffi.NewBigInt("1000000000000000000"), // 1 WETH
				})
			}

			if len(requests) >= 10 {
				break
			}
		}

		if len(requests) == 0 {
			log.Fatal("No WETH pools found at all")
		}
	}

	fmt.Printf("\nSimulating %d swaps in parallel...\n", len(requests))

	// Run batch simulation
	start := time.Now()
	resp, err := bridge.BatchGetAmountOut(ffi.BatchGetAmountOutRequest{
		Requests: requests,
	})
	elapsed := time.Since(start)

	if err != nil {
		log.Fatalf("Batch simulation failed: %v", err)
	}

	fmt.Printf("Batch completed in %v\n\n", elapsed)

	// Print results
	successCount := 0
	for i, result := range resp.Results {
		if result.Response != nil {
			successCount++
			amountOut := result.Response.AmountOut.ToBigInt()
			// Convert from USDC (6 decimals) to human readable
			usdcFloat := float64(amountOut.Int64()) / 1e6
			fmt.Printf("Pool %d: 1 WETH -> %.2f USDC\n", i+1, usdcFloat)
		} else {
			fmt.Printf("Pool %d: Error - %s\n", i+1, result.Error)
		}
	}

	fmt.Printf("\n%d/%d simulations successful\n", successCount, len(requests))
	fmt.Printf("Average time per simulation: %v\n", elapsed/time.Duration(len(requests)))
}
