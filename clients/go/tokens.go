package tychobridge

import (
	"context"
	"fmt"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// Token represents an ERC-20 token.
type Token struct {
	// Address is the hex-encoded address (e.g., "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").
	Address string

	// Decimals is the token decimals (e.g., 18).
	Decimals uint32

	// Symbol is the human-readable symbol (e.g., "WETH").
	Symbol string

	// Quality is the token quality score (0-100).
	// 100: Normal token, 75: Rebase token, 50: Fee token
	// 10: Analysis failed at creation, 5: Analysis failed on cronjob, 0: Failed to extract decimals
	Quality int32
}

// GetTokensRequest contains the parameters for querying tokens.
type GetTokensRequest struct {
	// Chain to query (required).
	Chain Chain

	// MinQuality filters by minimum token quality (0-100, defaults to 100).
	MinQuality *int32

	// TradedNDaysAgo filters to tokens traded within this many days.
	TradedNDaysAgo *uint64

	// Page is the page number for pagination (1-indexed).
	Page uint32

	// PageSize is the number of results per page.
	PageSize uint32
}

// GetTokensResponse contains the result of a GetTokens request.
type GetTokensResponse struct {
	// Tokens contains the matching tokens.
	Tokens []*Token

	// TotalCount is the total number of matching tokens (for pagination).
	TotalCount uint32
}

// GetTokens retrieves tokens from the local cache matching the filter criteria.
func (c *Client) GetTokens(ctx context.Context, req *GetTokensRequest) (*GetTokensResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.GetTokensRequest{
		Chain:    chainToProto(req.Chain),
		Page:     req.Page,
		PageSize: req.PageSize,
	}

	if req.MinQuality != nil {
		pbReq.MinQuality = req.MinQuality
	}
	if req.TradedNDaysAgo != nil {
		pbReq.TradedNDaysAgo = req.TradedNDaysAgo
	}

	resp, err := c.client.GetTokens(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("GetTokens failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	return &GetTokensResponse{
		Tokens:     convertTokens(resp.Tokens),
		TotalCount: resp.TotalCount,
	}, nil
}

// convertTokens converts protobuf Token messages to Go Token structs.
func convertTokens(pbTokens []*pb.Token) []*Token {
	tokens := make([]*Token, len(pbTokens))
	for i, t := range pbTokens {
		tokens[i] = convertToken(t)
	}
	return tokens
}

// convertToken converts a single protobuf Token to a Go Token.
func convertToken(t *pb.Token) *Token {
	if t == nil {
		return nil
	}
	return &Token{
		Address:  t.Address,
		Decimals: t.Decimals,
		Symbol:   t.Symbol,
		Quality:  t.Quality,
	}
}
