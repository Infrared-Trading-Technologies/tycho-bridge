// Package main benchmarks the performance of Tycho Bridge FFI simulation calls.
//
// This example measures the latency overhead of the FFI layer, including:
// - JSON serialization/deserialization costs
// - CGo call overhead
// - Rust execution time
//
// Run with: go run main.go
// Environment variables:
//   - TYCHO_API_KEY: API key for Tycho indexer
//   - TYCHO_URL: Tycho indexer URL (default: tycho-beta.propellerheads.xyz)
//   - BENCHMARK_ITERATIONS: Number of iterations per benchmark (default: 1000)
package main

import (
	"fmt"
	"log"
	"os"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"

	ffi "github.com/propeller-heads/tycho-bridge-ffi"
)

// BenchmarkResult holds timing statistics for a benchmark.
type BenchmarkResult struct {
	Name       string
	Iterations int
	TotalTime  time.Duration
	MinTime    time.Duration
	MaxTime    time.Duration
	MeanTime   time.Duration
	MedianTime time.Duration
	P95Time    time.Duration
	P99Time    time.Duration
}

func (r BenchmarkResult) String() string {
	opsPerSec := float64(r.Iterations) / r.TotalTime.Seconds()
	return fmt.Sprintf(`%s:
  Iterations:  %d
  Total Time:  %v
  Ops/sec:     %.2f
  Min:         %v
  Max:         %v
  Mean:        %v
  Median:      %v
  P95:         %v
  P99:         %v`,
		r.Name, r.Iterations, r.TotalTime, opsPerSec,
		r.MinTime, r.MaxTime, r.MeanTime, r.MedianTime, r.P95Time, r.P99Time)
}

func calculateStats(name string, durations []time.Duration) BenchmarkResult {
	if len(durations) == 0 {
		return BenchmarkResult{Name: name}
	}

	// Sort for percentile calculations
	sorted := make([]time.Duration, len(durations))
	copy(sorted, durations)
	sort.Slice(sorted, func(i, j int) bool { return sorted[i] < sorted[j] })

	var total time.Duration
	for _, d := range durations {
		total += d
	}

	n := len(sorted)
	p95Idx := int(float64(n) * 0.95)
	p99Idx := int(float64(n) * 0.99)
	if p95Idx >= n {
		p95Idx = n - 1
	}
	if p99Idx >= n {
		p99Idx = n - 1
	}

	return BenchmarkResult{
		Name:       name,
		Iterations: n,
		TotalTime:  total,
		MinTime:    sorted[0],
		MaxTime:    sorted[n-1],
		MeanTime:   total / time.Duration(n),
		MedianTime: sorted[n/2],
		P95Time:    sorted[p95Idx],
		P99Time:    sorted[p99Idx],
	}
}

func main() {
	// Configuration
	apiKey := os.Getenv("TYCHO_API_KEY")
	tychoURL := os.Getenv("TYCHO_URL")
	if tychoURL == "" {
		tychoURL = "tycho-beta.propellerheads.xyz"
	}

	iterations := 1000
	if iterStr := os.Getenv("BENCHMARK_ITERATIONS"); iterStr != "" {
		if n, err := strconv.Atoi(iterStr); err == nil && n > 0 {
			iterations = n
		}
	}

	fmt.Println("╔══════════════════════════════════════════════════════════════╗")
	fmt.Println("║           Tycho Bridge FFI Benchmark Suite                   ║")
	fmt.Println("╚══════════════════════════════════════════════════════════════╝")
	fmt.Printf("\nVersion: %s\n", ffi.Version())
	fmt.Printf("Go Version: %s\n", runtime.Version())
	fmt.Printf("GOMAXPROCS: %d\n", runtime.GOMAXPROCS(0))
	fmt.Printf("Iterations per benchmark: %d\n\n", iterations)

	// Initialize bridge
	config := ffi.Config{
		TychoURL:    tychoURL,
		TychoAPIKey: apiKey,
		ChainIDs:    []uint64{1}, // Ethereum mainnet
	}

	fmt.Println("Initializing bridge...")
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
	for i := 0; i < 60; i++ {
		time.Sleep(1 * time.Second)
		count := bridge.ComponentCount()
		fmt.Printf("\r  Components synced: %d", count)
		if count > 50 {
			break
		}
	}
	fmt.Println()

	componentCount := bridge.ComponentCount()
	if componentCount == 0 {
		log.Fatal("No components synced, cannot run benchmarks")
	}
	fmt.Printf("Ready with %d components\n\n", componentCount)

	// Find a suitable pool for benchmarking
	pool, tokenIn, tokenOut := findBenchmarkPool(bridge)
	if pool.ID == "" {
		log.Fatal("No suitable pool found for benchmarking")
	}

	fmt.Printf("Benchmark Pool: %s\n", pool.ID)
	fmt.Printf("Protocol: %s\n", pool.ProtocolSystem)
	fmt.Printf("Token Pair: %s -> %s\n\n", tokenIn[:10]+"...", tokenOut[:10]+"...")

	// Warm-up run
	fmt.Println("Warming up (100 calls)...")
	for i := 0; i < 100; i++ {
		_, _ = bridge.SpotPrice(ffi.SpotPriceRequest{
			ChainID:     1,
			ComponentID: pool.ID,
			BaseToken:   tokenIn,
			QuoteToken:  tokenOut,
		})
	}
	runtime.GC() // Clean up before benchmarks
	fmt.Println()

	// Run benchmarks
	results := []BenchmarkResult{}

	// Benchmark 1: SpotPrice
	fmt.Println("Running SpotPrice benchmark...")
	results = append(results, benchmarkSpotPrice(bridge, pool.ID, tokenIn, tokenOut, iterations))

	// Benchmark 2: GetAmountOut
	fmt.Println("Running GetAmountOut benchmark...")
	results = append(results, benchmarkGetAmountOut(bridge, pool.ID, tokenIn, tokenOut, iterations))

	// Benchmark 3: GetFee
	fmt.Println("Running GetFee benchmark...")
	results = append(results, benchmarkGetFee(bridge, pool.ID, tokenIn, iterations))

	// Benchmark 4: GetLimits
	fmt.Println("Running GetLimits benchmark...")
	results = append(results, benchmarkGetLimits(bridge, pool.ID, tokenIn, tokenOut, iterations))

	// Benchmark 5: GetComponents (query overhead)
	fmt.Println("Running GetComponents benchmark...")
	results = append(results, benchmarkGetComponents(bridge, iterations/10)) // Fewer iterations (heavier call)

	// Benchmark 6: BatchGetAmountOut with comparison
	fmt.Println("Running BatchGetAmountOut benchmark...")
	batchPools := findMultiplePools(bridge, 10)
	if len(batchPools) > 0 {
		fmt.Printf("  Found %d pools for batch testing\n", len(batchPools))
		batchResult, seqResult := benchmarkBatchVsSequential(bridge, batchPools, iterations/10)
		results = append(results, seqResult)
		results = append(results, batchResult)
	} else {
		fmt.Println("  No pools found for batch testing, skipping...")
	}

	// Benchmark 7: Health check (lightweight baseline)
	fmt.Println("Running HealthCheck benchmark (baseline)...")
	results = append(results, benchmarkHealthCheck(bridge, iterations))

	// Print results
	fmt.Println("\n" + strings.Repeat("═", 70))
	fmt.Println("BENCHMARK RESULTS")
	fmt.Println(strings.Repeat("═", 70))

	for _, r := range results {
		fmt.Println()
		fmt.Println(r)
	}

	// Summary table
	fmt.Println("\n" + strings.Repeat("═", 70))
	fmt.Println("SUMMARY (Mean Latency)")
	fmt.Println(strings.Repeat("═", 70))
	fmt.Printf("%-30s %15s %15s\n", "Operation", "Mean Latency", "Ops/sec")
	fmt.Println(strings.Repeat("-", 70))
	for _, r := range results {
		opsPerSec := float64(r.Iterations) / r.TotalTime.Seconds()
		fmt.Printf("%-30s %15v %15.2f\n", r.Name, r.MeanTime, opsPerSec)
	}

	// Analysis
	fmt.Println("\n" + strings.Repeat("═", 70))
	fmt.Println("ANALYSIS")
	fmt.Println(strings.Repeat("═", 70))
	fmt.Println(`
FFI Overhead Analysis:
- Pure CGo call overhead: ~50-100ns (baseline)
- JSON serialization (Rust): ~500ns-2μs depending on response size
- JSON deserialization (Go): ~500ns-2μs depending on response size
- Total expected overhead: ~1-5μs per call

If your mean latencies are:
- < 10μs: Excellent - minimal overhead
- 10-50μs: Good - acceptable for most use cases
- 50-100μs: Fair - consider batching for hot paths
- > 100μs: May indicate network/sync issues or large responses

For high-throughput scenarios, consider:
1. Using BatchGetAmountOut for multiple simulations
2. Caching component IDs to avoid repeated lookups
3. Running simulations in parallel from multiple goroutines
`)

	fmt.Println("\nBenchmark complete!")
}

func findBenchmarkPool(bridge *ffi.Bridge) (ffi.Component, string, string) {
	weth := "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
	usdc := "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
	wethLower := strings.ToLower(weth)
	usdcLower := strings.ToLower(usdc)

	components, err := bridge.GetComponents(ffi.GetComponentsRequest{
		ChainID: 1,
		Limit:   500,
	})
	if err != nil || components == nil {
		return ffi.Component{}, "", ""
	}

	// Prefer WETH/USDC pools (commonly used, stable)
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
			return c, weth, usdc
		}
	}

	// Fallback: any pool with 2 tokens
	for _, c := range components.Components {
		if len(c.Tokens) >= 2 {
			return c, c.Tokens[0], c.Tokens[1]
		}
	}

	return ffi.Component{}, "", ""
}

func findMultiplePools(bridge *ffi.Bridge, count int) []struct {
	pool     ffi.Component
	tokenIn  string
	tokenOut string
} {
	weth := "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
	usdc := "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
	wethLower := strings.ToLower(weth)
	usdcLower := strings.ToLower(usdc)

	components, err := bridge.GetComponents(ffi.GetComponentsRequest{
		ChainID: 1,
		Limit:   500,
	})
	if err != nil || components == nil {
		return nil
	}

	var pools []struct {
		pool     ffi.Component
		tokenIn  string
		tokenOut string
	}

	// First pass: find WETH/USDC pools (preferred)
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
			pools = append(pools, struct {
				pool     ffi.Component
				tokenIn  string
				tokenOut string
			}{c, weth, usdc})
		}
		if len(pools) >= count {
			return pools
		}
	}

	// Second pass: fall back to any pools with 2+ tokens
	for _, c := range components.Components {
		if len(c.Tokens) >= 2 {
			// Skip if already added
			alreadyAdded := false
			for _, p := range pools {
				if p.pool.ID == c.ID {
					alreadyAdded = true
					break
				}
			}
			if !alreadyAdded {
				pools = append(pools, struct {
					pool     ffi.Component
					tokenIn  string
					tokenOut string
				}{c, c.Tokens[0], c.Tokens[1]})
			}
		}
		if len(pools) >= count {
			break
		}
	}

	return pools
}

func benchmarkSpotPrice(bridge *ffi.Bridge, poolID, tokenIn, tokenOut string, iterations int) BenchmarkResult {
	durations := make([]time.Duration, 0, iterations)
	req := ffi.SpotPriceRequest{
		ChainID:     1,
		ComponentID: poolID,
		BaseToken:   tokenIn,
		QuoteToken:  tokenOut,
	}

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.SpotPrice(req)
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats("SpotPrice", durations)
}

func benchmarkGetAmountOut(bridge *ffi.Bridge, poolID, tokenIn, tokenOut string, iterations int) BenchmarkResult {
	durations := make([]time.Duration, 0, iterations)
	req := ffi.GetAmountOutRequest{
		ChainID:     1,
		ComponentID: poolID,
		TokenIn:     tokenIn,
		TokenOut:    tokenOut,
		AmountIn:    ffi.NewBigInt("1000000000000000000"), // 1 token
	}

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.GetAmountOut(req)
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats("GetAmountOut", durations)
}

func benchmarkGetFee(bridge *ffi.Bridge, poolID, token string, iterations int) BenchmarkResult {
	durations := make([]time.Duration, 0, iterations)
	req := ffi.GetFeeRequest{
		ChainID:     1,
		ComponentID: poolID,
		Token:       token,
	}

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.GetFee(req)
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats("GetFee", durations)
}

func benchmarkGetLimits(bridge *ffi.Bridge, poolID, tokenIn, tokenOut string, iterations int) BenchmarkResult {
	durations := make([]time.Duration, 0, iterations)
	req := ffi.GetLimitsRequest{
		ChainID:     1,
		ComponentID: poolID,
		TokenIn:     tokenIn,
		TokenOut:    tokenOut,
	}

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.GetLimits(req)
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats("GetLimits", durations)
}

func benchmarkGetComponents(bridge *ffi.Bridge, iterations int) BenchmarkResult {
	durations := make([]time.Duration, 0, iterations)
	req := ffi.GetComponentsRequest{
		ChainID: 1,
		Limit:   100,
	}

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.GetComponents(req)
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats("GetComponents (100)", durations)
}

func benchmarkBatchGetAmountOut(bridge *ffi.Bridge, pools []struct {
	pool     ffi.Component
	tokenIn  string
	tokenOut string
}, iterations int) BenchmarkResult {
	if len(pools) == 0 {
		return BenchmarkResult{Name: "BatchGetAmountOut (skipped)"}
	}

	durations := make([]time.Duration, 0, iterations)

	// Build batch request
	requests := make([]ffi.GetAmountOutRequest, len(pools))
	for i, p := range pools {
		requests[i] = ffi.GetAmountOutRequest{
			ChainID:     1,
			ComponentID: p.pool.ID,
			TokenIn:     p.tokenIn,
			TokenOut:    p.tokenOut,
			AmountIn:    ffi.NewBigInt("1000000000000000000"),
		}
	}

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.BatchGetAmountOut(ffi.BatchGetAmountOutRequest{
			Requests: requests,
		})
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats(fmt.Sprintf("BatchGetAmountOut (%d pools)", len(pools)), durations)
}

// benchmarkBatchVsSequential compares batch vs sequential performance
func benchmarkBatchVsSequential(bridge *ffi.Bridge, pools []struct {
	pool     ffi.Component
	tokenIn  string
	tokenOut string
}, iterations int) (batchResult, seqResult BenchmarkResult) {
	if len(pools) == 0 {
		return BenchmarkResult{Name: "BatchGetAmountOut (skipped)"},
			BenchmarkResult{Name: "Sequential (skipped)"}
	}

	numPools := len(pools)

	// Build requests
	requests := make([]ffi.GetAmountOutRequest, numPools)
	for i, p := range pools {
		requests[i] = ffi.GetAmountOutRequest{
			ChainID:     1,
			ComponentID: p.pool.ID,
			TokenIn:     p.tokenIn,
			TokenOut:    p.tokenOut,
			AmountIn:    ffi.NewBigInt("1000000000000000000"),
		}
	}

	// Benchmark sequential calls (N individual calls)
	seqDurations := make([]time.Duration, 0, iterations)
	for i := 0; i < iterations; i++ {
		start := time.Now()
		for _, req := range requests {
			_, _ = bridge.GetAmountOut(req)
		}
		elapsed := time.Since(start)
		seqDurations = append(seqDurations, elapsed)
	}
	seqResult = calculateStats(fmt.Sprintf("Sequential (%d calls)", numPools), seqDurations)

	// Benchmark batch call (1 call for N pools)
	batchDurations := make([]time.Duration, 0, iterations)
	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.BatchGetAmountOut(ffi.BatchGetAmountOutRequest{
			Requests: requests,
		})
		elapsed := time.Since(start)
		if err == nil {
			batchDurations = append(batchDurations, elapsed)
		}
	}
	batchResult = calculateStats(fmt.Sprintf("BatchGetAmountOut (%d pools)", numPools), batchDurations)

	// Print comparison
	fmt.Println()
	fmt.Println(strings.Repeat("-", 70))
	fmt.Printf("BATCH vs SEQUENTIAL COMPARISON (%d pools)\n", numPools)
	fmt.Println(strings.Repeat("-", 70))
	fmt.Printf("  Sequential (%d × GetAmountOut):  %v total, %v per call\n",
		numPools, seqResult.MeanTime, seqResult.MeanTime/time.Duration(numPools))
	fmt.Printf("  Batch (1 × BatchGetAmountOut):   %v total, %v per pool\n",
		batchResult.MeanTime, batchResult.MeanTime/time.Duration(numPools))

	if seqResult.MeanTime > 0 && batchResult.MeanTime > 0 {
		speedup := float64(seqResult.MeanTime) / float64(batchResult.MeanTime)
		overhead := seqResult.MeanTime - batchResult.MeanTime
		fmt.Printf("  Speedup: %.2fx faster with batching\n", speedup)
		fmt.Printf("  FFI overhead saved: %v (%d calls → 1 call)\n", overhead, numPools)
	}
	fmt.Println(strings.Repeat("-", 70))

	return batchResult, seqResult
}

func benchmarkHealthCheck(bridge *ffi.Bridge, iterations int) BenchmarkResult {
	durations := make([]time.Duration, 0, iterations)

	for i := 0; i < iterations; i++ {
		start := time.Now()
		_, err := bridge.HealthCheck()
		elapsed := time.Since(start)
		if err == nil {
			durations = append(durations, elapsed)
		}
	}

	return calculateStats("HealthCheck (baseline)", durations)
}
