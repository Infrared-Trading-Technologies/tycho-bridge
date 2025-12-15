package tychobridge

import (
	"context"
	"fmt"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// BigInt represents an unsigned 256-bit integer as a decimal string.
type BigInt struct {
	Value string
}

// NewBigInt creates a new BigInt from a string value.
func NewBigInt(value string) *BigInt {
	return &BigInt{Value: value}
}

// NewBigIntFromUint64 creates a new BigInt from a uint64 value.
func NewBigIntFromUint64(value uint64) *BigInt {
	return &BigInt{Value: fmt.Sprintf("%d", value)}
}

// GetAmountOutRequest contains the parameters for simulating a swap.
type GetAmountOutRequest struct {
	// Chain to simulate on (required).
	Chain Chain

	// ComponentID is the pool/component ID to simulate against.
	ComponentID string

	// AmountIn is the input amount.
	AmountIn *BigInt

	// TokenIn is the input token address.
	TokenIn string

	// TokenOut is the output token address.
	TokenOut string

	// PoolStateOverride is an optional override for the pool state.
	// Pass the NewPoolState from a previous simulation to chain simulations.
	PoolStateOverride []byte
}

// GetAmountOutResponse contains the result of a swap simulation.
type GetAmountOutResponse struct {
	// Amount is the expected output amount.
	Amount *BigInt

	// Gas is the estimated gas cost.
	Gas *BigInt

	// NewPoolState is the serialized pool state after the swap.
	// Can be passed to subsequent simulations for chained swaps.
	NewPoolState []byte

	// ExecutionTimeUs is the total execution time in microseconds (includes all overhead).
	ExecutionTimeUs uint64

	// SimulationTimeUs is the pure AMM simulation time in microseconds (just get_amount_out).
	SimulationTimeUs uint64

	// EffectivePrice is the effective price (amount_out / amount_in).
	EffectivePrice float64

	// PriceImpactBPS is the price impact in basis points.
	PriceImpactBPS float64
}

// GetAmountOut executes a local swap simulation against cached pool state.
// This operation makes NO external network calls - all math runs in-memory.
func (c *Client) GetAmountOut(ctx context.Context, req *GetAmountOutRequest) (*GetAmountOutResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.GetAmountOutRequest{
		Chain:       chainToProto(req.Chain),
		ComponentId: req.ComponentID,
		TokenIn:     req.TokenIn,
		TokenOut:    req.TokenOut,
	}

	if req.AmountIn != nil {
		pbReq.AmountIn = &pb.BigInt{Value: req.AmountIn.Value}
	}

	if len(req.PoolStateOverride) > 0 {
		pbReq.PoolStateOverride = req.PoolStateOverride
	}

	resp, err := c.client.GetAmountOut(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("GetAmountOut failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &GetAmountOutResponse{
		Amount:           convertBigInt(resp.Amount),
		Gas:              convertBigInt(resp.Gas),
		NewPoolState:     resp.NewPoolState,
		ExecutionTimeUs:  resp.ExecutionTimeUs,
		SimulationTimeUs: resp.SimulationTimeUs,
		EffectivePrice:   resp.EffectivePrice,
		PriceImpactBPS:   resp.PriceImpactBps,
	}, nil
}

// convertBigInt converts a protobuf BigInt to a Go BigInt.
func convertBigInt(b *pb.BigInt) *BigInt {
	if b == nil {
		return nil
	}
	return &BigInt{Value: b.Value}
}

// SpotPriceRequest contains the parameters for querying a spot price.
type SpotPriceRequest struct {
	// Chain to query (required).
	Chain Chain

	// ComponentID is the pool/component ID to query.
	ComponentID string

	// BaseToken is the token you're selling.
	BaseToken string

	// QuoteToken is the token you're buying.
	QuoteToken string
}

// Price represents a price as a rational fraction.
type Price struct {
	Numerator   *BigInt
	Denominator *BigInt
}

// SpotPriceResponse contains the result of a spot price query.
type SpotPriceResponse struct {
	// Price is the marginal price as a float (quote_token per base_token).
	Price float64

	// PriceRational is the price as a rational for exact math.
	PriceRational *Price
}

// SpotPrice returns the pool's current marginal price.
func (c *Client) SpotPrice(ctx context.Context, req *SpotPriceRequest) (*SpotPriceResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.SpotPriceRequest{
		Chain:       chainToProto(req.Chain),
		ComponentId: req.ComponentID,
		BaseToken:   req.BaseToken,
		QuoteToken:  req.QuoteToken,
	}

	resp, err := c.client.SpotPrice(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("SpotPrice failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &SpotPriceResponse{
		Price:         resp.Price,
		PriceRational: convertPrice(resp.PriceRational),
	}, nil
}

// convertPrice converts a protobuf Price to a Go Price.
func convertPrice(p *pb.Price) *Price {
	if p == nil {
		return nil
	}
	return &Price{
		Numerator:   convertBigInt(p.Numerator),
		Denominator: convertBigInt(p.Denominator),
	}
}

// GetFeeRequest contains the parameters for querying a protocol fee.
type GetFeeRequest struct {
	// Chain to query (required).
	Chain Chain

	// ComponentID is the pool/component ID to query.
	ComponentID string
}

// GetFeeResponse contains the result of a fee query.
type GetFeeResponse struct {
	// Fee is the fee as a ratio (e.g., 0.003 for 0.3%).
	Fee float64

	// FeeBPS is the fee in basis points (e.g., 30 for 0.3%).
	FeeBPS uint32

	// IsDynamic indicates whether this is a dynamic fee.
	IsDynamic bool
}

// GetFee returns the protocol fee for a component.
func (c *Client) GetFee(ctx context.Context, req *GetFeeRequest) (*GetFeeResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.GetFeeRequest{
		Chain:       chainToProto(req.Chain),
		ComponentId: req.ComponentID,
	}

	resp, err := c.client.GetFee(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("GetFee failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &GetFeeResponse{
		Fee:       resp.Fee,
		FeeBPS:    resp.FeeBps,
		IsDynamic: resp.IsDynamic,
	}, nil
}

// GetLimitsRequest contains the parameters for querying trading limits.
type GetLimitsRequest struct {
	// Chain to query (required).
	Chain Chain

	// ComponentID is the pool/component ID to query.
	ComponentID string

	// SellToken is the token being sold.
	SellToken string

	// BuyToken is the token being bought.
	BuyToken string
}

// GetLimitsResponse contains the result of a limits query.
type GetLimitsResponse struct {
	// MaxSellAmount is the maximum amount of sell_token that can be traded.
	MaxSellAmount *BigInt

	// MaxBuyAmount is the maximum amount of buy_token that can be received.
	MaxBuyAmount *BigInt

	// IsSoftLimit indicates whether these are soft limits (advised maximums).
	IsSoftLimit bool
}

// GetLimits returns the maximum tradeable amounts between two tokens.
func (c *Client) GetLimits(ctx context.Context, req *GetLimitsRequest) (*GetLimitsResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.GetLimitsRequest{
		Chain:       chainToProto(req.Chain),
		ComponentId: req.ComponentID,
		SellToken:   req.SellToken,
		BuyToken:    req.BuyToken,
	}

	resp, err := c.client.GetLimits(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("GetLimits failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &GetLimitsResponse{
		MaxSellAmount: convertBigInt(resp.MaxSellAmount),
		MaxBuyAmount:  convertBigInt(resp.MaxBuyAmount),
		IsSoftLimit:   resp.IsSoftLimit,
	}, nil
}

// Trade represents a trade result.
type Trade struct {
	AmountIn  *BigInt
	AmountOut *BigInt
}

// SwapToPriceRequest contains the parameters for a price-targeting trade calculation.
type SwapToPriceRequest struct {
	// Chain to query (required).
	Chain Chain

	// ComponentID is the pool/component ID to query.
	ComponentID string

	// TokenIn is the token being sold.
	TokenIn string

	// TokenOut is the token being bought.
	TokenOut string

	// TargetPrice is the target price as token_out/token_in, net of all fees.
	TargetPrice *Price
}

// SwapToPriceResponse contains the result of a price-targeting trade calculation.
type SwapToPriceResponse struct {
	// Trade is the trade required to reach target price.
	Trade *Trade

	// IsReachable indicates whether the target price is reachable.
	IsReachable bool
}

// SwapToPrice calculates the trade required to move pool price to a target.
func (c *Client) SwapToPrice(ctx context.Context, req *SwapToPriceRequest) (*SwapToPriceResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.SwapToPriceRequest{
		Chain:       chainToProto(req.Chain),
		ComponentId: req.ComponentID,
		TokenIn:     req.TokenIn,
		TokenOut:    req.TokenOut,
	}

	if req.TargetPrice != nil {
		pbReq.TargetPrice = priceToProto(req.TargetPrice)
	}

	resp, err := c.client.SwapToPrice(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("SwapToPrice failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &SwapToPriceResponse{
		Trade:       convertTrade(resp.Trade),
		IsReachable: resp.IsReachable,
	}, nil
}

// priceToProto converts a Go Price to a protobuf Price.
func priceToProto(p *Price) *pb.Price {
	if p == nil {
		return nil
	}
	result := &pb.Price{}
	if p.Numerator != nil {
		result.Numerator = &pb.BigInt{Value: p.Numerator.Value}
	}
	if p.Denominator != nil {
		result.Denominator = &pb.BigInt{Value: p.Denominator.Value}
	}
	return result
}

// convertTrade converts a protobuf Trade to a Go Trade.
func convertTrade(t *pb.Trade) *Trade {
	if t == nil {
		return nil
	}
	return &Trade{
		AmountIn:  convertBigInt(t.AmountIn),
		AmountOut: convertBigInt(t.AmountOut),
	}
}

// QuerySupplyRequest contains the parameters for a supply curve query.
type QuerySupplyRequest struct {
	// Chain to query (required).
	Chain Chain

	// ComponentID is the pool/component ID to query.
	ComponentID string

	// TokenIn is the token being sold.
	TokenIn string

	// TokenOut is the token being bought.
	TokenOut string

	// MinPrice is the minimum acceptable price (token_out/token_in), net of all fees.
	MinPrice *Price
}

// QuerySupplyResponse contains the result of a supply curve query.
type QuerySupplyResponse struct {
	// Trade is the maximum trade that maintains price above min_price.
	Trade *Trade
}

// QuerySupply returns the maximum token_out a pool can supply at a price floor.
func (c *Client) QuerySupply(ctx context.Context, req *QuerySupplyRequest) (*QuerySupplyResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.QuerySupplyRequest{
		Chain:       chainToProto(req.Chain),
		ComponentId: req.ComponentID,
		TokenIn:     req.TokenIn,
		TokenOut:    req.TokenOut,
	}

	if req.MinPrice != nil {
		pbReq.MinPrice = priceToProto(req.MinPrice)
	}

	resp, err := c.client.QuerySupply(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("QuerySupply failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &QuerySupplyResponse{
		Trade: convertTrade(resp.Trade),
	}, nil
}

// BatchGetAmountOutRequest contains the parameters for batch simulations.
type BatchGetAmountOutRequest struct {
	// Requests are the individual simulation requests.
	Requests []*GetAmountOutRequest

	// Parallel indicates whether to execute in parallel (default: true).
	Parallel bool
}

// SimulationError represents an error from simulation.
type SimulationError struct {
	Code    SimulationErrorCode
	Message string
}

// SimulationErrorCode represents simulation error codes.
type SimulationErrorCode int32

const (
	SimulationErrorCodeUnspecified           SimulationErrorCode = 0
	SimulationErrorCodeComponentNotFound     SimulationErrorCode = 1
	SimulationErrorCodeTokenNotFound         SimulationErrorCode = 2
	SimulationErrorCodeInsufficientLiquidity SimulationErrorCode = 3
	SimulationErrorCodeAmountExceedsLimit    SimulationErrorCode = 4
	SimulationErrorCodeInvalidTokenPair      SimulationErrorCode = 5
	SimulationErrorCodePoolStateError        SimulationErrorCode = 6
	SimulationErrorCodeVMExecutionError      SimulationErrorCode = 7
)

// GetAmountOutResult wraps a single simulation result with error handling.
type GetAmountOutResult struct {
	Success *GetAmountOutResponse
	Error   *SimulationError
}

// BatchGetAmountOutResponse contains the result of batch simulations.
type BatchGetAmountOutResponse struct {
	// Results are the individual simulation results.
	Results []*GetAmountOutResult

	// TotalExecutionTimeUs is the total execution time in microseconds.
	TotalExecutionTimeUs uint64
}

// BatchGetAmountOut executes multiple simulations in parallel.
func (c *Client) BatchGetAmountOut(ctx context.Context, req *BatchGetAmountOutRequest) (*BatchGetAmountOutResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbRequests := make([]*pb.GetAmountOutRequest, len(req.Requests))
	for i, r := range req.Requests {
		pbRequests[i] = &pb.GetAmountOutRequest{
			Chain:       chainToProto(r.Chain),
			ComponentId: r.ComponentID,
			TokenIn:     r.TokenIn,
			TokenOut:    r.TokenOut,
		}
		if r.AmountIn != nil {
			pbRequests[i].AmountIn = &pb.BigInt{Value: r.AmountIn.Value}
		}
		if len(r.PoolStateOverride) > 0 {
			pbRequests[i].PoolStateOverride = r.PoolStateOverride
		}
	}

	pbReq := &pb.BatchGetAmountOutRequest{
		Requests: pbRequests,
		Parallel: req.Parallel,
	}

	resp, err := c.client.BatchGetAmountOut(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("BatchGetAmountOut failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	results := make([]*GetAmountOutResult, len(resp.Results))
	for i, r := range resp.Results {
		results[i] = convertGetAmountOutResult(r)
	}

	return &BatchGetAmountOutResponse{
		Results:              results,
		TotalExecutionTimeUs: resp.TotalExecutionTimeUs,
	}, nil
}

// convertGetAmountOutResult converts a protobuf GetAmountOutResult to Go.
func convertGetAmountOutResult(r *pb.GetAmountOutResult) *GetAmountOutResult {
	if r == nil {
		return nil
	}

	result := &GetAmountOutResult{}

	switch v := r.Result.(type) {
	case *pb.GetAmountOutResult_Success:
		result.Success = &GetAmountOutResponse{
			Amount:          convertBigInt(v.Success.Amount),
			Gas:             convertBigInt(v.Success.Gas),
			NewPoolState:    v.Success.NewPoolState,
			ExecutionTimeUs: v.Success.ExecutionTimeUs,
			EffectivePrice:  v.Success.EffectivePrice,
			PriceImpactBPS:  v.Success.PriceImpactBps,
		}
	case *pb.GetAmountOutResult_Error:
		result.Error = &SimulationError{
			Code:    SimulationErrorCode(v.Error.Code),
			Message: v.Error.Message,
		}
	}

	return result
}
