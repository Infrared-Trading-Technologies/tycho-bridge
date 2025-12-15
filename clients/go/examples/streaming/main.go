package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	tychobridge "github.com/branched-services/tycho-bridge-go"
)

func main() {
	addr := os.Getenv("TYCHO_BRIDGE_ADDR")
	if addr == "" {
		addr = "localhost:50051"
	}

	client, err := tychobridge.NewClient(addr)
	if err != nil {
		log.Fatalf("Failed to create client: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigChan
		fmt.Println("\nShutting down...")
		cancel()
	}()

	// Subscribe to real-time updates using the new StreamUpdates API
	stream, err := client.StreamUpdatesWithReconnect(ctx, &tychobridge.StreamUpdatesRequest{
		Chain:           tychobridge.ChainEthereum,
		ProtocolSystems: []string{"uniswap_v2", "uniswap_v3"},
	})
	if err != nil {
		log.Fatalf("Failed to start stream: %v", err)
	}
	defer stream.Close()

	fmt.Println("Subscribed to protocol updates. Press Ctrl+C to exit.")
	fmt.Println()

	var newCount, modifiedCount, removedCount int
	startTime := time.Now()

	for {
		select {
		case update := <-stream.Updates():
			if update == nil {
				fmt.Println("Stream closed")
				return
			}

			// Count new components
			newCount += len(update.NewComponents)

			// Count state updates (modified)
			modifiedCount += len(update.StateUpdates)

			// Count removed components
			removedCount += len(update.RemovedComponentIDs)

			fmt.Printf("[Block %d] new=%d modified=%d removed=%d\n",
				update.BlockNumber,
				len(update.NewComponents),
				len(update.StateUpdates),
				len(update.RemovedComponentIDs),
			)

		case err := <-stream.Errors():
			if err != nil {
				log.Printf("Stream error (will reconnect): %v", err)
			}

		case <-ctx.Done():
			elapsed := time.Since(startTime)
			fmt.Println()
			fmt.Printf("Summary after %v:\n", elapsed.Round(time.Second))
			fmt.Printf("  New components: %d\n", newCount)
			fmt.Printf("  Modified components: %d\n", modifiedCount)
			fmt.Printf("  Removed components: %d\n", removedCount)
			fmt.Printf("  Total updates: %d\n", newCount+modifiedCount+removedCount)
			return
		}
	}
}
