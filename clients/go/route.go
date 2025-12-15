package tychobridge

import (
	"context"
	"fmt"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// RouteHop represents a single hop in a multi-hop route.
type RouteHop struct {
	// ComponentID is the pool/component ID for this hop.
	ComponentID string

	// TokenIn is the token being sold in this hop.
	TokenIn string

	// TokenOut is the token being bought in this hop.
	TokenOut string
}

// SimulateRouteRequest contains the parameters for multi-hop route simulation.
type SimulateRouteRequest struct {
	// Chain to simulate on (required).
	Chain Chain

	// Hops are the ordered list of hops in the route.
	Hops []*RouteHop

	// AmountIn is the initial amount to swap.
	AmountIn *BigInt

	// PoolStateOverrides are optional override pool states for speculative simulation.
	PoolStateOverrides map[string][]byte
}

// HopResult contains the result of a single hop in a route.
type HopResult struct {
	// ComponentID is the pool/component ID for this hop.
	ComponentID string

	// AmountIn is the input amount for this hop.
	AmountIn *BigInt

	// AmountOut is the output amount for this hop.
	AmountOut *BigInt

	// Gas is the estimated gas for this hop.
	Gas *BigInt

	// EffectivePrice is the effective price for this hop.
	EffectivePrice float64

	// PriceImpactBPS is the price impact in basis points for this hop.
	PriceImpactBPS float64
}

// SimulateRouteResponse contains the result of a multi-hop route simulation.
type SimulateRouteResponse struct {
	// AmountOut is the final amount received after all hops.
	AmountOut *BigInt

	// TotalGas is the total estimated gas for all hops.
	TotalGas *BigInt

	// Hops are the results for each hop.
	Hops []*HopResult

	// FinalPoolStates are the pool states after all swaps (for further simulations).
	FinalPoolStates map[string][]byte

	// TotalPriceImpactBPS is the total price impact across all hops (in basis points).
	TotalPriceImpactBPS float64

	// TotalExecutionTimeUs is the total execution time in microseconds.
	TotalExecutionTimeUs uint64
}

// SimulateRoute simulates a multi-hop swap path with state transitions.
func (c *Client) SimulateRoute(ctx context.Context, req *SimulateRouteRequest) (*SimulateRouteResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbHops := make([]*pb.RouteHop, len(req.Hops))
	for i, hop := range req.Hops {
		pbHops[i] = &pb.RouteHop{
			ComponentId: hop.ComponentID,
			TokenIn:     hop.TokenIn,
			TokenOut:    hop.TokenOut,
		}
	}

	pbReq := &pb.SimulateRouteRequest{
		Chain:              chainToProto(req.Chain),
		Hops:               pbHops,
		PoolStateOverrides: req.PoolStateOverrides,
	}

	if req.AmountIn != nil {
		pbReq.AmountIn = &pb.BigInt{Value: req.AmountIn.Value}
	}

	resp, err := c.client.SimulateRoute(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("SimulateRoute failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	hops := make([]*HopResult, len(resp.Hops))
	for i, hop := range resp.Hops {
		hops[i] = &HopResult{
			ComponentID:    hop.ComponentId,
			AmountIn:       convertBigInt(hop.AmountIn),
			AmountOut:      convertBigInt(hop.AmountOut),
			Gas:            convertBigInt(hop.Gas),
			EffectivePrice: hop.EffectivePrice,
			PriceImpactBPS: hop.PriceImpactBps,
		}
	}

	return &SimulateRouteResponse{
		AmountOut:            convertBigInt(resp.AmountOut),
		TotalGas:             convertBigInt(resp.TotalGas),
		Hops:                 hops,
		FinalPoolStates:      resp.FinalPoolStates,
		TotalPriceImpactBPS:  resp.TotalPriceImpactBps,
		TotalExecutionTimeUs: resp.TotalExecutionTimeUs,
	}, nil
}
