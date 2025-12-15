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

// GetTokensRequest contains parameters for querying tokens.
type GetTokensRequest struct {
	// ChainID filters by chain (0 = all chains).
	ChainID uint64

	// Limit is the maximum number of tokens to return (0 = no limit).
	Limit uint32
}

// GetTokensResponse contains the result of a GetTokens request.
type GetTokensResponse struct {
	// Tokens contains the matching tokens.
	Tokens []Token

	// TotalCount is the total number of matching tokens.
	TotalCount uint32
}

// getTokensJSON is the JSON response from the FFI.
type getTokensJSON struct {
	Tokens []struct {
		Address  string `json:"address"`
		Symbol   string `json:"symbol"`
		Decimals uint32 `json:"decimals"`
		Quality  int32  `json:"quality"`
	} `json:"tokens"`
	TotalCount uint32 `json:"total_count"`
}

// GetTokens retrieves tokens from the local cache.
func (b *Bridge) GetTokens(req GetTokensRequest) (*GetTokensResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cReq := C.FfiGetTokensRequest{
		chain_id: C.uint64_t(req.ChainID),
		limit:    C.uint32_t(req.Limit),
	}

	result := C.tycho_get_tokens(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp getTokensJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	response := &GetTokensResponse{
		Tokens:     make([]Token, len(resp.Tokens)),
		TotalCount: resp.TotalCount,
	}

	for i, t := range resp.Tokens {
		response.Tokens[i] = Token{
			Address:  t.Address,
			Symbol:   t.Symbol,
			Decimals: t.Decimals,
			Quality:  t.Quality,
		}
	}

	return response, nil
}

// GetComponentsRequest contains parameters for querying components.
type GetComponentsRequest struct {
	// ChainID filters by chain (required).
	ChainID uint64

	// ProtocolType filters by protocol type (e.g., "uniswap_v2", empty = all).
	ProtocolType string

	// MinTVL filters by minimum TVL in USD (0 = no filter).
	MinTVL float64

	// Limit is the maximum number of components to return (0 = no limit).
	Limit uint32
}

// GetComponentsResponse contains the result of a GetComponents request.
type GetComponentsResponse struct {
	// Components contains the matching components.
	Components []Component

	// TotalCount is the total number of matching components.
	TotalCount uint32
}

// getComponentsJSON is the JSON response from the FFI.
type getComponentsJSON struct {
	Components []struct {
		ID             string   `json:"id"`
		ProtocolSystem string   `json:"protocol_system"`
		ProtocolType   string   `json:"protocol_type"`
		Tokens         []string `json:"tokens"`
		TVL            float64  `json:"tvl"`
		ChainID        uint64   `json:"chain_id"`
	} `json:"components"`
	TotalCount uint32 `json:"total_count"`
}

// GetComponents retrieves protocol components from the local cache.
func (b *Bridge) GetComponents(req GetComponentsRequest) (*GetComponentsResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	var cProtocolType *C.char
	if req.ProtocolType != "" {
		cProtocolType = C.CString(req.ProtocolType)
		defer C.free(unsafe.Pointer(cProtocolType))
	}

	cReq := C.FfiGetComponentsRequest{
		chain_id:      C.uint64_t(req.ChainID),
		protocol_type: cProtocolType,
		min_tvl:       C.double(req.MinTVL),
		limit:         C.uint32_t(req.Limit),
	}

	result := C.tycho_get_components(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp getComponentsJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	response := &GetComponentsResponse{
		Components: make([]Component, len(resp.Components)),
		TotalCount: resp.TotalCount,
	}

	// If Rust didn't return total_count, use component count
	if response.TotalCount == 0 {
		response.TotalCount = uint32(len(resp.Components))
	}

	for i, c := range resp.Components {
		response.Components[i] = Component{
			ID:             c.ID,
			ProtocolSystem: c.ProtocolSystem,
			ProtocolType:   c.ProtocolType,
			Tokens:         c.Tokens,
			TVL:            c.TVL,
			ChainID:        c.ChainID,
		}
	}

	return response, nil
}
