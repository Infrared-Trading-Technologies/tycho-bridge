package tychobridge

import (
	"context"
	"fmt"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// ProtocolSyncStatus provides sync status for a protocol.
type ProtocolSyncStatus struct {
	// ProtocolSystem is the protocol system name.
	ProtocolSystem string

	// Status is the sync status: "ready", "delayed", "stale", "ended".
	Status string

	// LatestBlock is the latest synced block for this protocol.
	LatestBlock uint64
}

// ChainStatus provides status for a specific chain.
type ChainStatus struct {
	// Chain is the chain this status is for.
	Chain Chain

	// Synced indicates whether the chain is synced.
	Synced bool

	// LatestBlock is the latest block number in cache.
	LatestBlock uint64

	// ComponentCount is the number of protocol components tracked.
	ComponentCount uint64

	// TokenCount is the number of tokens tracked.
	TokenCount uint64

	// IndexerURL is the Tycho Indexer URL.
	IndexerURL string

	// ProtocolSyncStatuses provides sync status per protocol system.
	ProtocolSyncStatuses []*ProtocolSyncStatus
}

// StatusResponse contains the health and status of the Tycho Bridge service.
type StatusResponse struct {
	// Healthy indicates the service is ready to serve requests.
	Healthy bool

	// Version is the service version.
	Version string

	// Chains provides status for each chain being served.
	Chains []*ChainStatus

	// UptimeSeconds is the uptime in seconds.
	UptimeSeconds uint64
}

// GetStatus returns the health and cache statistics of the service.
func (c *Client) GetStatus(ctx context.Context) (*StatusResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	resp, err := c.client.GetStatus(ctx, &pb.GetStatusRequest{})
	if err != nil {
		return nil, fmt.Errorf("GetStatus failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &StatusResponse{
		Healthy:       resp.Healthy,
		Version:       resp.Version,
		Chains:        convertChainStatuses(resp.Chains),
		UptimeSeconds: resp.UptimeSeconds,
	}, nil
}

// convertChainStatuses converts protobuf ChainStatus messages to Go.
func convertChainStatuses(pbChains []*pb.ChainStatus) []*ChainStatus {
	chains := make([]*ChainStatus, len(pbChains))
	for i, c := range pbChains {
		chains[i] = convertChainStatus(c)
	}
	return chains
}

// convertChainStatus converts a protobuf ChainStatus to Go.
func convertChainStatus(c *pb.ChainStatus) *ChainStatus {
	if c == nil {
		return nil
	}

	return &ChainStatus{
		Chain:                protoToChain(c.Chain),
		Synced:               c.Synced,
		LatestBlock:          c.LatestBlock,
		ComponentCount:       c.ComponentCount,
		TokenCount:           c.TokenCount,
		IndexerURL:           c.IndexerUrl,
		ProtocolSyncStatuses: convertProtocolSyncStatuses(c.ProtocolSyncStatuses),
	}
}

// convertProtocolSyncStatuses converts protobuf ProtocolSyncStatus messages to Go.
func convertProtocolSyncStatuses(pbStatuses []*pb.ProtocolSyncStatus) []*ProtocolSyncStatus {
	statuses := make([]*ProtocolSyncStatus, len(pbStatuses))
	for i, s := range pbStatuses {
		statuses[i] = &ProtocolSyncStatus{
			ProtocolSystem: s.ProtocolSystem,
			Status:         s.Status,
			LatestBlock:    s.LatestBlock,
		}
	}
	return statuses
}
