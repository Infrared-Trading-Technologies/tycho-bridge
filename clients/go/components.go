package tychobridge

import (
	"context"
	"fmt"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// Chain represents a blockchain network.
type Chain int32

const (
	ChainUnspecified Chain = 0
	ChainEthereum    Chain = 1
	ChainBase        Chain = 2
	ChainUnichain    Chain = 3
	ChainArbitrum    Chain = 4
)

// chainToProto converts a Chain enum to the protobuf Chain enum.
func chainToProto(c Chain) pb.Chain {
	return pb.Chain(c)
}

// protoToChain converts a protobuf Chain enum to the Go Chain enum.
func protoToChain(c pb.Chain) Chain {
	return Chain(c)
}

// GetProtocolComponentsRequest contains the parameters for querying protocol components.
type GetProtocolComponentsRequest struct {
	// Chain to query (required).
	Chain Chain

	// ProtocolSystems filters by protocol systems (e.g., "uniswap_v2", "uniswap_v3").
	ProtocolSystems []string

	// ComponentIDs filters by specific component IDs.
	ComponentIDs []string

	// TokenAddresses filters to components containing ANY of these tokens.
	TokenAddresses []string

	// MinTVL filters to components with TVL >= this value (in native token).
	MinTVL *float64

	// Page is the page number for pagination (1-indexed).
	Page uint32

	// PageSize is the number of results per page.
	PageSize uint32
}

// GetProtocolComponentsResponse contains the result of a GetProtocolComponents request.
type GetProtocolComponentsResponse struct {
	// Components contains the matching protocol components.
	Components []*ProtocolComponent

	// TotalCount is the total number of matching components (for pagination).
	TotalCount uint32
}

// ProtocolComponent represents a protocol component (pool, market, etc.).
type ProtocolComponent struct {
	// ID is the unique identifier (typically the pool contract address).
	ID string

	// ProtocolSystem is the protocol system name (e.g., "uniswap_v2", "uniswap_v3").
	ProtocolSystem string

	// ProtocolTypeName is the protocol type (e.g., "UniswapV2Pool", "UniswapV3Pool").
	ProtocolTypeName string

	// Chain is the chain this component exists on.
	Chain Chain

	// Tokens are the token addresses in this component.
	Tokens []string

	// ContractAddresses are the contract addresses involved.
	ContractAddresses []string

	// StaticAttributes are static attributes from the protocol (fee tier, weights, etc.).
	StaticAttributes map[string]string

	// CreationTx is the transaction hash that created this component.
	CreationTx string
}

// GetProtocolComponents retrieves protocol components from the local cache matching the filter criteria.
func (c *Client) GetProtocolComponents(ctx context.Context, req *GetProtocolComponentsRequest) (*GetProtocolComponentsResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.GetProtocolComponentsRequest{
		Chain:           chainToProto(req.Chain),
		ProtocolSystems: req.ProtocolSystems,
		ComponentIds:    req.ComponentIDs,
		TokenAddresses:  req.TokenAddresses,
		Page:            req.Page,
		PageSize:        req.PageSize,
	}

	if req.MinTVL != nil {
		pbReq.MinTvl = req.MinTVL
	}

	resp, err := c.client.GetProtocolComponents(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("GetProtocolComponents failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &GetProtocolComponentsResponse{
		Components: convertProtocolComponents(resp.Components),
		TotalCount: resp.TotalCount,
	}, nil
}

// convertProtocolComponents converts protobuf ProtocolComponent messages to Go ProtocolComponent structs.
func convertProtocolComponents(pbComponents []*pb.ProtocolComponent) []*ProtocolComponent {
	components := make([]*ProtocolComponent, len(pbComponents))
	for i, c := range pbComponents {
		components[i] = convertProtocolComponent(c)
	}
	return components
}

// convertProtocolComponent converts a single protobuf ProtocolComponent to a Go ProtocolComponent.
func convertProtocolComponent(c *pb.ProtocolComponent) *ProtocolComponent {
	if c == nil {
		return nil
	}
	return &ProtocolComponent{
		ID:                c.Id,
		ProtocolSystem:    c.ProtocolSystem,
		ProtocolTypeName:  c.ProtocolTypeName,
		Chain:             protoToChain(c.Chain),
		Tokens:            c.Tokens,
		ContractAddresses: c.ContractAddresses,
		StaticAttributes:  c.StaticAttributes,
		CreationTx:        c.CreationTx,
	}
}
