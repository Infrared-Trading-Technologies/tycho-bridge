package tychobridge

import (
	"testing"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

func TestChainToProto(t *testing.T) {
	tests := []struct {
		chain    Chain
		expected pb.Chain
	}{
		{ChainUnspecified, pb.Chain_CHAIN_UNSPECIFIED},
		{ChainEthereum, pb.Chain_CHAIN_ETHEREUM},
		{ChainBase, pb.Chain_CHAIN_BASE},
		{ChainUnichain, pb.Chain_CHAIN_UNICHAIN},
		// Note: ChainArbitrum is defined but generated proto not yet regenerated
	}

	for _, tt := range tests {
		got := chainToProto(tt.chain)
		if got != tt.expected {
			t.Errorf("chainToProto(%v) = %v, want %v", tt.chain, got, tt.expected)
		}
	}
}

func TestProtoToChain(t *testing.T) {
	tests := []struct {
		pb       pb.Chain
		expected Chain
	}{
		{pb.Chain_CHAIN_UNSPECIFIED, ChainUnspecified},
		{pb.Chain_CHAIN_ETHEREUM, ChainEthereum},
		{pb.Chain_CHAIN_BASE, ChainBase},
		{pb.Chain_CHAIN_UNICHAIN, ChainUnichain},
		// Note: ChainArbitrum is defined but generated proto not yet regenerated
	}

	for _, tt := range tests {
		got := protoToChain(tt.pb)
		if got != tt.expected {
			t.Errorf("protoToChain(%v) = %v, want %v", tt.pb, got, tt.expected)
		}
	}
}

func TestConvertProtocolComponent(t *testing.T) {
	// Test nil component
	if got := convertProtocolComponent(nil); got != nil {
		t.Errorf("convertProtocolComponent(nil) = %v, want nil", got)
	}

	// Test full component conversion
	pbComponent := &pb.ProtocolComponent{
		Id:                "0x123",
		ProtocolSystem:    "uniswap_v2",
		ProtocolTypeName:  "UniswapV2Pool",
		Chain:             pb.Chain_CHAIN_ETHEREUM,
		Tokens:            []string{"0xabc", "0xdef"},
		ContractAddresses: []string{"0x456"},
		StaticAttributes:  map[string]string{"fee": "3000"},
		CreationTx:        "0x789",
	}

	component := convertProtocolComponent(pbComponent)
	if component.ID != "0x123" {
		t.Errorf("component.ID = %q, want %q", component.ID, "0x123")
	}
	if component.ProtocolSystem != "uniswap_v2" {
		t.Errorf("component.ProtocolSystem = %q, want %q", component.ProtocolSystem, "uniswap_v2")
	}
	if component.ProtocolTypeName != "UniswapV2Pool" {
		t.Errorf("component.ProtocolTypeName = %q, want %q", component.ProtocolTypeName, "UniswapV2Pool")
	}
	if component.Chain != ChainEthereum {
		t.Errorf("component.Chain = %v, want %v", component.Chain, ChainEthereum)
	}
	if len(component.Tokens) != 2 {
		t.Errorf("len(component.Tokens) = %d, want %d", len(component.Tokens), 2)
	}
	if component.Tokens[0] != "0xabc" {
		t.Errorf("component.Tokens[0] = %q, want %q", component.Tokens[0], "0xabc")
	}
	if len(component.ContractAddresses) != 1 {
		t.Errorf("len(component.ContractAddresses) = %d, want %d", len(component.ContractAddresses), 1)
	}
	if component.StaticAttributes["fee"] != "3000" {
		t.Errorf("component.StaticAttributes[\"fee\"] = %q, want %q", component.StaticAttributes["fee"], "3000")
	}
	if component.CreationTx != "0x789" {
		t.Errorf("component.CreationTx = %q, want %q", component.CreationTx, "0x789")
	}
}

func TestConvertProtocolComponents(t *testing.T) {
	pbComponents := []*pb.ProtocolComponent{
		{Id: "0x111", ProtocolSystem: "uniswap_v2", Chain: pb.Chain_CHAIN_ETHEREUM},
		{Id: "0x222", ProtocolSystem: "uniswap_v3", Chain: pb.Chain_CHAIN_BASE},
	}

	components := convertProtocolComponents(pbComponents)
	if len(components) != 2 {
		t.Errorf("len(components) = %d, want %d", len(components), 2)
	}
	if components[0].ID != "0x111" {
		t.Errorf("components[0].ID = %q, want %q", components[0].ID, "0x111")
	}
	if components[1].Chain != ChainBase {
		t.Errorf("components[1].Chain = %v, want %v", components[1].Chain, ChainBase)
	}
}
