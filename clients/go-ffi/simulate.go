package tychoffi

/*
#cgo CFLAGS: -I${SRCDIR}/../../include
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -ltycho_bridge -ldl -lm -lpthread
#include "tycho_bridge.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"unsafe"
)

// SpotPriceRequest contains the parameters for querying a spot price.
type SpotPriceRequest struct {
	// ChainID is the chain ID (e.g., 1 for Ethereum).
	ChainID uint64

	// ComponentID is the pool/component ID to query.
	ComponentID string

	// BaseToken is the token you're selling.
	BaseToken string

	// QuoteToken is the token you're buying.
	QuoteToken string
}

// SpotPriceResponse contains the result of a spot price query.
type SpotPriceResponse struct {
	// Price is the marginal price as a rational fraction.
	Price *Price

	// PriceFloat is the price as a float64 for convenience.
	PriceFloat float64
}

// spotPriceJSON is the JSON response from the FFI.
type spotPriceJSON struct {
	Numerator   string `json:"numerator"`
	Denominator string `json:"denominator"`
}

// SpotPrice queries the marginal price for a token pair in a pool.
func (b *Bridge) SpotPrice(req SpotPriceRequest) (*SpotPriceResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cComponentID := C.CString(req.ComponentID)
	defer C.free(unsafe.Pointer(cComponentID))
	cBaseToken := C.CString(req.BaseToken)
	defer C.free(unsafe.Pointer(cBaseToken))
	cQuoteToken := C.CString(req.QuoteToken)
	defer C.free(unsafe.Pointer(cQuoteToken))

	cReq := C.FfiSpotPriceRequest{
		chain_id:     C.uint64_t(req.ChainID),
		component_id: cComponentID,
		base_token:   cBaseToken,
		quote_token:  cQuoteToken,
	}

	result := C.tycho_spot_price(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp spotPriceJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	price := &Price{
		Numerator:   NewBigInt(resp.Numerator),
		Denominator: NewBigInt(resp.Denominator),
	}

	return &SpotPriceResponse{
		Price:      price,
		PriceFloat: price.ToFloat64(),
	}, nil
}

// GetAmountOutRequest contains the parameters for simulating a swap.
type GetAmountOutRequest struct {
	// ChainID is the chain ID (e.g., 1 for Ethereum).
	ChainID uint64

	// ComponentID is the pool/component ID to simulate against.
	ComponentID string

	// TokenIn is the input token address.
	TokenIn string

	// TokenOut is the output token address.
	TokenOut string

	// AmountIn is the input amount.
	AmountIn *BigInt

	// StateID is an optional state override ID from a previous simulation.
	StateID string

	// SaveState indicates whether to save the resulting state.
	SaveState bool
}

// GetAmountOutResponse contains the result of a swap simulation.
type GetAmountOutResponse struct {
	// AmountOut is the expected output amount.
	AmountOut *BigInt

	// GasCost is the estimated gas cost.
	GasCost *BigInt

	// StateID is the ID of the saved state (if SaveState was true).
	StateID string

	// ExecutionPrice is the effective price (amount_out / amount_in).
	ExecutionPrice *Price
}

// getAmountOutJSON is the JSON response from the FFI.
type getAmountOutJSON struct {
	AmountOut                 string `json:"amount_out"`
	GasCost                   string `json:"gas_cost"`
	StateID                   string `json:"state_id,omitempty"`
	ExecutionPriceNumerator   string `json:"execution_price_numerator,omitempty"`
	ExecutionPriceDenominator string `json:"execution_price_denominator,omitempty"`
}

// GetAmountOut simulates a swap and returns the expected output.
func (b *Bridge) GetAmountOut(req GetAmountOutRequest) (*GetAmountOutResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cComponentID := C.CString(req.ComponentID)
	defer C.free(unsafe.Pointer(cComponentID))
	cTokenIn := C.CString(req.TokenIn)
	defer C.free(unsafe.Pointer(cTokenIn))
	cTokenOut := C.CString(req.TokenOut)
	defer C.free(unsafe.Pointer(cTokenOut))

	amountInStr := "0"
	if req.AmountIn != nil {
		amountInStr = req.AmountIn.Value
	}
	cAmountIn := C.CString(amountInStr)
	defer C.free(unsafe.Pointer(cAmountIn))

	var cStateID *C.char
	if req.StateID != "" {
		cStateID = C.CString(req.StateID)
		defer C.free(unsafe.Pointer(cStateID))
	}

	cReq := C.FfiGetAmountOutRequest{
		chain_id:     C.uint64_t(req.ChainID),
		component_id: cComponentID,
		token_in:     cTokenIn,
		token_out:    cTokenOut,
		amount_in:    cAmountIn,
		state_id:     cStateID,
		save_state:   C.bool(req.SaveState),
	}

	result := C.tycho_get_amount_out(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp getAmountOutJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	response := &GetAmountOutResponse{
		AmountOut: NewBigInt(resp.AmountOut),
		GasCost:   NewBigInt(resp.GasCost),
		StateID:   resp.StateID,
	}

	if resp.ExecutionPriceNumerator != "" && resp.ExecutionPriceDenominator != "" {
		response.ExecutionPrice = &Price{
			Numerator:   NewBigInt(resp.ExecutionPriceNumerator),
			Denominator: NewBigInt(resp.ExecutionPriceDenominator),
		}
	}

	return response, nil
}

// GetFeeRequest contains the parameters for querying a pool fee.
type GetFeeRequest struct {
	// ChainID is the chain ID.
	ChainID uint64

	// ComponentID is the pool/component ID.
	ComponentID string

	// Token is the token address to query the fee for.
	Token string
}

// GetFeeResponse contains the fee information.
type GetFeeResponse struct {
	// Fee is the fee as a rational fraction.
	Fee *Price
}

// getFeeJSON is the JSON response from the FFI.
type getFeeJSON struct {
	FeeNumerator   string `json:"fee_numerator"`
	FeeDenominator string `json:"fee_denominator"`
}

// GetFee queries the fee for a pool/token.
func (b *Bridge) GetFee(req GetFeeRequest) (*GetFeeResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cComponentID := C.CString(req.ComponentID)
	defer C.free(unsafe.Pointer(cComponentID))
	cToken := C.CString(req.Token)
	defer C.free(unsafe.Pointer(cToken))

	cReq := C.FfiGetFeeRequest{
		chain_id:     C.uint64_t(req.ChainID),
		component_id: cComponentID,
		token:        cToken,
	}

	result := C.tycho_get_fee(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp getFeeJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	return &GetFeeResponse{
		Fee: &Price{
			Numerator:   NewBigInt(resp.FeeNumerator),
			Denominator: NewBigInt(resp.FeeDenominator),
		},
	}, nil
}

// GetLimitsRequest contains the parameters for querying trading limits.
type GetLimitsRequest struct {
	// ChainID is the chain ID.
	ChainID uint64

	// ComponentID is the pool/component ID.
	ComponentID string

	// TokenIn is the input token address.
	TokenIn string

	// TokenOut is the output token address.
	TokenOut string
}

// GetLimitsResponse contains the trading limits.
type GetLimitsResponse struct {
	// MinAmountIn is the minimum input amount.
	MinAmountIn *BigInt

	// MaxAmountIn is the maximum input amount.
	MaxAmountIn *BigInt
}

// getLimitsJSON is the JSON response from the FFI.
type getLimitsJSON struct {
	MinAmountIn string `json:"min_amount_in"`
	MaxAmountIn string `json:"max_amount_in"`
}

// GetLimits queries the trading limits for a pool.
func (b *Bridge) GetLimits(req GetLimitsRequest) (*GetLimitsResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cComponentID := C.CString(req.ComponentID)
	defer C.free(unsafe.Pointer(cComponentID))
	cTokenIn := C.CString(req.TokenIn)
	defer C.free(unsafe.Pointer(cTokenIn))
	cTokenOut := C.CString(req.TokenOut)
	defer C.free(unsafe.Pointer(cTokenOut))

	cReq := C.FfiGetLimitsRequest{
		chain_id:     C.uint64_t(req.ChainID),
		component_id: cComponentID,
		token_in:     cTokenIn,
		token_out:    cTokenOut,
	}

	result := C.tycho_get_limits(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp getLimitsJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	return &GetLimitsResponse{
		MinAmountIn: NewBigInt(resp.MinAmountIn),
		MaxAmountIn: NewBigInt(resp.MaxAmountIn),
	}, nil
}

// SwapToPriceRequest contains parameters for calculating a swap to reach a target price.
type SwapToPriceRequest struct {
	// ChainID is the chain ID.
	ChainID uint64

	// ComponentID is the pool/component ID.
	ComponentID string

	// TokenIn is the input token address.
	TokenIn string

	// TokenOut is the output token address.
	TokenOut string

	// TargetPrice is the target price to reach.
	TargetPrice *Price
}

// SwapToPriceResponse contains the result of a swap-to-price calculation.
type SwapToPriceResponse struct {
	// AmountIn is the input amount needed.
	AmountIn *BigInt

	// AmountOut is the expected output amount.
	AmountOut *BigInt

	// AchievedPrice is the price after the swap.
	AchievedPrice *Price
}

// swapToPriceJSON is the JSON response from the FFI.
type swapToPriceJSON struct {
	AmountIn                 string `json:"amount_in"`
	AmountOut                string `json:"amount_out"`
	AchievedPriceNumerator   string `json:"achieved_price_numerator"`
	AchievedPriceDenominator string `json:"achieved_price_denominator"`
}

// SwapToPrice calculates the swap needed to reach a target price.
func (b *Bridge) SwapToPrice(req SwapToPriceRequest) (*SwapToPriceResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	if req.TargetPrice == nil || req.TargetPrice.Numerator == nil || req.TargetPrice.Denominator == nil {
		return nil, ErrInvalidArgument
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cComponentID := C.CString(req.ComponentID)
	defer C.free(unsafe.Pointer(cComponentID))
	cTokenIn := C.CString(req.TokenIn)
	defer C.free(unsafe.Pointer(cTokenIn))
	cTokenOut := C.CString(req.TokenOut)
	defer C.free(unsafe.Pointer(cTokenOut))
	cNumerator := C.CString(req.TargetPrice.Numerator.Value)
	defer C.free(unsafe.Pointer(cNumerator))
	cDenominator := C.CString(req.TargetPrice.Denominator.Value)
	defer C.free(unsafe.Pointer(cDenominator))

	cReq := C.FfiSwapToPriceRequest{
		chain_id:                 C.uint64_t(req.ChainID),
		component_id:             cComponentID,
		token_in:                 cTokenIn,
		token_out:                cTokenOut,
		target_price_numerator:   cNumerator,
		target_price_denominator: cDenominator,
	}

	result := C.tycho_swap_to_price(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp swapToPriceJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	return &SwapToPriceResponse{
		AmountIn:  NewBigInt(resp.AmountIn),
		AmountOut: NewBigInt(resp.AmountOut),
		AchievedPrice: &Price{
			Numerator:   NewBigInt(resp.AchievedPriceNumerator),
			Denominator: NewBigInt(resp.AchievedPriceDenominator),
		},
	}, nil
}

// RouteHop represents a single hop in a multi-hop route.
type RouteHop struct {
	// ComponentID is the pool/component ID for this hop.
	ComponentID string

	// TokenIn is the input token address.
	TokenIn string

	// TokenOut is the output token address.
	TokenOut string
}

// SimulateRouteRequest contains parameters for simulating a multi-hop route.
type SimulateRouteRequest struct {
	// ChainID is the chain ID.
	ChainID uint64

	// Hops is the list of hops in the route.
	Hops []RouteHop

	// AmountIn is the initial input amount.
	AmountIn *BigInt
}

// HopResult contains the result of a single hop in the route.
type HopResult struct {
	// ComponentID is the pool/component ID.
	ComponentID string

	// AmountIn is the input amount for this hop.
	AmountIn *BigInt

	// AmountOut is the output amount for this hop.
	AmountOut *BigInt

	// GasCost is the gas cost for this hop.
	GasCost *BigInt
}

// SimulateRouteResponse contains the result of a route simulation.
type SimulateRouteResponse struct {
	// FinalAmountOut is the final output amount.
	FinalAmountOut *BigInt

	// TotalGasCost is the total gas cost for all hops.
	TotalGasCost *BigInt

	// HopResults contains results for each hop.
	HopResults []HopResult
}

// simulateRouteJSON is the JSON response from the FFI.
type simulateRouteJSON struct {
	FinalAmountOut string `json:"final_amount_out"`
	TotalGasCost   string `json:"total_gas_cost"`
	HopResults     []struct {
		ComponentID string `json:"component_id"`
		AmountIn    string `json:"amount_in"`
		AmountOut   string `json:"amount_out"`
		GasCost     string `json:"gas_cost"`
	} `json:"hop_results"`
}

// SimulateRoute simulates a multi-hop route.
func (b *Bridge) SimulateRoute(req SimulateRouteRequest) (*SimulateRouteResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	if len(req.Hops) == 0 {
		return nil, ErrInvalidArgument
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	// Allocate C arrays for hops
	cHops := make([]C.FfiRouteHop, len(req.Hops))
	cStrings := make([]*C.char, 0, len(req.Hops)*3) // Keep track for cleanup

	for i, hop := range req.Hops {
		cComponentID := C.CString(hop.ComponentID)
		cTokenIn := C.CString(hop.TokenIn)
		cTokenOut := C.CString(hop.TokenOut)

		cStrings = append(cStrings, cComponentID, cTokenIn, cTokenOut)

		cHops[i] = C.FfiRouteHop{
			component_id: cComponentID,
			token_in:     cTokenIn,
			token_out:    cTokenOut,
		}
	}

	// Cleanup strings
	defer func() {
		for _, s := range cStrings {
			C.free(unsafe.Pointer(s))
		}
	}()

	amountInStr := "0"
	if req.AmountIn != nil {
		amountInStr = req.AmountIn.Value
	}
	cAmountIn := C.CString(amountInStr)
	defer C.free(unsafe.Pointer(cAmountIn))

	cReq := C.FfiSimulateRouteRequest{
		chain_id:  C.uint64_t(req.ChainID),
		hops:      &cHops[0],
		hop_count: C.uintptr_t(len(cHops)),
		amount_in: cAmountIn,
	}

	result := C.tycho_simulate_route(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp simulateRouteJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	response := &SimulateRouteResponse{
		FinalAmountOut: NewBigInt(resp.FinalAmountOut),
		TotalGasCost:   NewBigInt(resp.TotalGasCost),
		HopResults:     make([]HopResult, len(resp.HopResults)),
	}

	for i, hr := range resp.HopResults {
		response.HopResults[i] = HopResult{
			ComponentID: hr.ComponentID,
			AmountIn:    NewBigInt(hr.AmountIn),
			AmountOut:   NewBigInt(hr.AmountOut),
			GasCost:     NewBigInt(hr.GasCost),
		}
	}

	return response, nil
}

// BatchGetAmountOutRequest contains parameters for batch swap simulations.
type BatchGetAmountOutRequest struct {
	// Requests is the list of individual GetAmountOut requests.
	Requests []GetAmountOutRequest
}

// BatchGetAmountOutResponse contains the results of batch swap simulations.
type BatchGetAmountOutResponse struct {
	// Results contains the result for each request (in order).
	// If a simulation failed, the corresponding result will have an error.
	Results []BatchGetAmountOutResult
}

// BatchGetAmountOutResult contains the result or error for a single batch item.
type BatchGetAmountOutResult struct {
	// Response is the simulation response (nil if Error is set).
	Response *GetAmountOutResponse

	// Error is the error message (empty if successful).
	Error string
}

// batchGetAmountOutJSON is the JSON response from the FFI.
type batchGetAmountOutJSON struct {
	Results []json.RawMessage `json:"results"`
}

// batchResultItemJSON is a single result item (can be null or object).
type batchResultItemJSON struct {
	AmountOut string `json:"amount_out"`
	GasCost   string `json:"gas_cost"`
	StateID   string `json:"state_id,omitempty"`
}

// BatchGetAmountOut simulates multiple swaps in parallel.
func (b *Bridge) BatchGetAmountOut(req BatchGetAmountOutRequest) (*BatchGetAmountOutResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	if len(req.Requests) == 0 {
		return &BatchGetAmountOutResponse{Results: []BatchGetAmountOutResult{}}, nil
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	// Allocate C array for requests (in C memory to avoid CGo pointer restrictions)
	reqSize := unsafe.Sizeof(C.FfiGetAmountOutRequest{})
	cRequestsPtr := (*C.FfiGetAmountOutRequest)(C.malloc(C.ulong(len(req.Requests)) * C.ulong(reqSize)))
	defer C.free(unsafe.Pointer(cRequestsPtr))

	// Create a slice backed by C memory
	cRequests := unsafe.Slice(cRequestsPtr, len(req.Requests))

	cStrings := make([]*C.char, 0, len(req.Requests)*5) // Keep track for cleanup

	for i, r := range req.Requests {
		cComponentID := C.CString(r.ComponentID)
		cTokenIn := C.CString(r.TokenIn)
		cTokenOut := C.CString(r.TokenOut)

		amountInStr := "0"
		if r.AmountIn != nil {
			amountInStr = r.AmountIn.Value
		}
		cAmountIn := C.CString(amountInStr)

		cStrings = append(cStrings, cComponentID, cTokenIn, cTokenOut, cAmountIn)

		var cStateID *C.char
		if r.StateID != "" {
			cStateID = C.CString(r.StateID)
			cStrings = append(cStrings, cStateID)
		}

		cRequests[i] = C.FfiGetAmountOutRequest{
			chain_id:     C.uint64_t(r.ChainID),
			component_id: cComponentID,
			token_in:     cTokenIn,
			token_out:    cTokenOut,
			amount_in:    cAmountIn,
			state_id:     cStateID,
			save_state:   C.bool(r.SaveState),
		}
	}

	// Cleanup strings
	defer func() {
		for _, s := range cStrings {
			C.free(unsafe.Pointer(s))
		}
	}()

	cReq := C.FfiBatchGetAmountOutRequest{
		requests: cRequestsPtr,
		count:    C.uintptr_t(len(req.Requests)),
	}

	result := C.tycho_batch_get_amount_out(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp batchGetAmountOutJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	response := &BatchGetAmountOutResponse{
		Results: make([]BatchGetAmountOutResult, len(resp.Results)),
	}

	for i, r := range resp.Results {
		// Check if result is null (indicating failure)
		if string(r) == "null" {
			response.Results[i] = BatchGetAmountOutResult{
				Error: "simulation failed",
			}
		} else {
			var item batchResultItemJSON
			if err := json.Unmarshal(r, &item); err != nil {
				response.Results[i] = BatchGetAmountOutResult{
					Error: "failed to parse result: " + err.Error(),
				}
			} else {
				response.Results[i] = BatchGetAmountOutResult{
					Response: &GetAmountOutResponse{
						AmountOut: NewBigInt(item.AmountOut),
						GasCost:   NewBigInt(item.GasCost),
						StateID:   item.StateID,
					},
				}
			}
		}
	}

	return response, nil
}
