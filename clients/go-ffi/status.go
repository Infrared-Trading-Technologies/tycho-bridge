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
)

// GetStatusRequest contains parameters for querying indexer status.
type GetStatusRequest struct {
	// ChainID filters by chain.
	ChainID uint64

	// IncludeChainDetails includes detailed per-chain status.
	IncludeChainDetails bool
}

// GetStatusResponse contains the indexer sync status.
type GetStatusResponse struct {
	// Status contains the indexer status information.
	Status IndexerStatus
}

// getStatusJSON is the JSON response from the FFI.
type getStatusJSON struct {
	SyncStatus     string `json:"sync_status"`
	LatestBlock    uint64 `json:"latest_block"`
	ComponentCount uint64 `json:"component_count"`
	UptimeSeconds  uint64 `json:"uptime_seconds"`
}

// GetStatus retrieves the indexer sync status.
func (b *Bridge) GetStatus(req GetStatusRequest) (*GetStatusResponse, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	cReq := C.FfiGetStatusRequest{
		chain_id:              C.uint64_t(req.ChainID),
		include_chain_details: C.bool(req.IncludeChainDetails),
	}

	result := C.tycho_get_status(b.handle, &cReq)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp getStatusJSON
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	return &GetStatusResponse{
		Status: IndexerStatus{
			SyncStatus:     resp.SyncStatus,
			LatestBlock:    resp.LatestBlock,
			ComponentCount: resp.ComponentCount,
			UptimeSeconds:  resp.UptimeSeconds,
		},
	}, nil
}

// HealthCheck performs a health check and returns the status.
func (b *Bridge) HealthCheck() (*HealthStatus, error) {
	if err := b.ensureOpen(); err != nil {
		return nil, err
	}

	b.mu.RLock()
	defer b.mu.RUnlock()

	result := C.tycho_health_check(b.handle)
	defer C.tycho_free_result(result)

	data, err := parseResult(result)
	if err != nil {
		return nil, err
	}

	var resp struct {
		Healthy bool   `json:"healthy"`
		Message string `json:"message"`
	}
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, NewBridgeError(int32(ErrorInternalError), "failed to parse response: "+err.Error())
	}

	return &HealthStatus{
		Healthy: resp.Healthy,
		Message: resp.Message,
	}, nil
}
