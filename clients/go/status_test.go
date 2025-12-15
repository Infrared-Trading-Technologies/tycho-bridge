package tychobridge

import (
	"testing"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

func TestConvertChainStatus(t *testing.T) {
	// Test nil chain status
	if got := convertChainStatus(nil); got != nil {
		t.Errorf("convertChainStatus(nil) should return nil, got %v", got)
	}

	// Test full chain status conversion
	pbStatus := &pb.ChainStatus{
		Chain:          pb.Chain_CHAIN_ETHEREUM,
		Synced:         true,
		LatestBlock:    12345678,
		ComponentCount: 100,
		TokenCount:     50,
		IndexerUrl:     "http://indexer.example.com",
		ProtocolSyncStatuses: []*pb.ProtocolSyncStatus{
			{
				ProtocolSystem: "uniswap_v2",
				Status:         "ready",
				LatestBlock:    12345670,
			},
		},
	}

	status := convertChainStatus(pbStatus)
	if status.Chain != ChainEthereum {
		t.Errorf("status.Chain = %v, want %v", status.Chain, ChainEthereum)
	}
	if status.LatestBlock != 12345678 {
		t.Errorf("status.LatestBlock = %d, want %d", status.LatestBlock, 12345678)
	}
	if !status.Synced {
		t.Error("status.Synced = false, want true")
	}
	if status.ComponentCount != 100 {
		t.Errorf("status.ComponentCount = %d, want %d", status.ComponentCount, 100)
	}
	if status.TokenCount != 50 {
		t.Errorf("status.TokenCount = %d, want %d", status.TokenCount, 50)
	}
	if len(status.ProtocolSyncStatuses) != 1 {
		t.Errorf("len(status.ProtocolSyncStatuses) = %d, want %d", len(status.ProtocolSyncStatuses), 1)
	}
	if status.ProtocolSyncStatuses[0].ProtocolSystem != "uniswap_v2" {
		t.Errorf("status.ProtocolSyncStatuses[0].ProtocolSystem = %q, want %q", status.ProtocolSyncStatuses[0].ProtocolSystem, "uniswap_v2")
	}
}

func TestConvertProtocolSyncStatus(t *testing.T) {
	pbStatus := &pb.ProtocolSyncStatus{
		ProtocolSystem: "uniswap_v3",
		Status:         "ready",
		LatestBlock:    12345600,
	}

	statuses := convertProtocolSyncStatuses([]*pb.ProtocolSyncStatus{pbStatus})
	if len(statuses) != 1 {
		t.Errorf("len(statuses) = %d, want %d", len(statuses), 1)
	}
	status := statuses[0]
	if status.ProtocolSystem != "uniswap_v3" {
		t.Errorf("status.ProtocolSystem = %q, want %q", status.ProtocolSystem, "uniswap_v3")
	}
	if status.Status != "ready" {
		t.Errorf("status.Status = %q, want %q", status.Status, "ready")
	}
	if status.LatestBlock != 12345600 {
		t.Errorf("status.LatestBlock = %d, want %d", status.LatestBlock, 12345600)
	}
}

func TestConvertChainStatuses(t *testing.T) {
	pbStatuses := []*pb.ChainStatus{
		{Chain: pb.Chain_CHAIN_ETHEREUM, LatestBlock: 12345678, Synced: true},
		{Chain: pb.Chain_CHAIN_BASE, LatestBlock: 87654321, Synced: false},
	}

	statuses := convertChainStatuses(pbStatuses)
	if len(statuses) != 2 {
		t.Errorf("len(statuses) = %d, want %d", len(statuses), 2)
	}
	if statuses[0].Chain != ChainEthereum {
		t.Errorf("statuses[0].Chain = %v, want %v", statuses[0].Chain, ChainEthereum)
	}
	if statuses[1].Chain != ChainBase {
		t.Errorf("statuses[1].Chain = %v, want %v", statuses[1].Chain, ChainBase)
	}
}
