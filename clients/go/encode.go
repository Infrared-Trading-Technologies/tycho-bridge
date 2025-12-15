package tychobridge

import (
	"context"
	"fmt"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// EncoderType specifies which encoder to use.
type EncoderType int32

const (
	EncoderTypeUnspecified   EncoderType = 0
	EncoderTypeTychoRouter   EncoderType = 1
	EncoderTypeTychoExecutor EncoderType = 2
)

// UserTransferType defines how funds are transferred from the user.
type UserTransferType int32

const (
	UserTransferTypeUnspecified         UserTransferType = 0
	UserTransferTypeTransferFromPermit2 UserTransferType = 1
	UserTransferTypeTransferFrom        UserTransferType = 2
	UserTransferTypeNone                UserTransferType = 3
)

// NativeAction specifies wrapping/unwrapping of native tokens.
type NativeAction int32

const (
	NativeActionUnspecified NativeAction = 0
	NativeActionWrap        NativeAction = 1
	NativeActionUnwrap      NativeAction = 2
)

// Swap represents a swap operation on a pool.
type Swap struct {
	// Component is the protocol component (pool) to swap on.
	Component *ProtocolComponent

	// TokenIn is the token you provide to the pool.
	TokenIn string

	// TokenOut is the token you expect from the pool.
	TokenOut string

	// Split is the percentage of amount_in for this swap (0.0-1.0).
	// Use 0 for remainder (router sends all remaining tokens).
	Split float64

	// ProtocolState is the optional protocol state for the swap.
	ProtocolState []byte

	// EstimatedAmountIn is the optional estimated amount_in (required for RFQ protocols).
	EstimatedAmountIn *BigInt
}

// Solution specifies the details of an order and how it should be filled.
type Solution struct {
	// GivenToken is the token being sold (exact in) or bought (exact out).
	GivenToken string

	// GivenAmount is the amount of the given token.
	GivenAmount *BigInt

	// CheckedToken is the token being bought - final balance will be checked by router.
	CheckedToken string

	// Sender is the address of the sender of given_token.
	Sender string

	// Receiver is the address of the receiver of checked_token.
	Receiver string

	// ExactOut is false for exact input (sell order), true for exact output (buy order).
	ExactOut bool

	// RouterAddress is the address of the Tycho Router contract.
	RouterAddress string

	// Swaps are the list of swaps to fulfill the solution.
	Swaps []*Swap

	// CheckedAmount is the minimum amount out for the solution to be valid.
	CheckedAmount *BigInt

	// NativeAction specifies wrapping/unwrapping of native tokens.
	NativeAction *NativeAction
}

// Permit represents Permit2 signature data.
type Permit struct {
	Token       string
	Amount      *BigInt
	Expiration  uint64
	Nonce       uint64
	Spender     string
	SigDeadline uint64
}

// EncodeSolutionRequest contains the parameters for encoding a solution.
type EncodeSolutionRequest struct {
	// Chain to encode for (required).
	Chain Chain

	// Solution is the solution to encode.
	Solution *Solution

	// EncoderType specifies which encoder to use.
	EncoderType EncoderType

	// UserTransferType defines how funds are transferred from the user.
	UserTransferType UserTransferType
}

// EncodeSolutionResponse contains the encoded solution.
type EncodeSolutionResponse struct {
	// EncodedSwaps is the encoded calldata for the swaps.
	EncodedSwaps []byte

	// InteractingWith is the contract address to call (Tycho Router or Executor).
	InteractingWith string

	// Selector is the function selector to call.
	Selector string

	// NTokens is the number of tokens in the trade.
	NTokens uint32

	// Permit is the optional Permit2 data (if using Permit2 transfers).
	Permit *Permit

	// FullCalldata is the full transaction data (for convenience - includes all guardrails).
	// WARNING: You should build your own calldata for production.
	FullCalldata []byte
}

// EncodeSolution converts a Solution into calldata for the Tycho Router.
func (c *Client) EncodeSolution(ctx context.Context, req *EncodeSolutionRequest) (*EncodeSolutionResponse, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	ctx, cancel := c.withTimeout(ctx)
	defer cancel()

	pbReq := &pb.EncodeSolutionRequest{
		Chain:            chainToProto(req.Chain),
		EncoderType:      pb.EncoderType(req.EncoderType),
		UserTransferType: pb.UserTransferType(req.UserTransferType),
	}

	if req.Solution != nil {
		pbReq.Solution = solutionToProto(req.Solution)
	}

	resp, err := c.client.EncodeSolution(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("EncodeSolution failed: %w", err)
	}

	if resp == nil {
		return nil, ErrNilResponse
	}

	result := &EncodeSolutionResponse{
		EncodedSwaps:    resp.EncodedSwaps,
		InteractingWith: resp.InteractingWith,
		Selector:        resp.Selector,
		NTokens:         resp.NTokens,
	}

	if resp.Permit != nil {
		result.Permit = &Permit{
			Token:       resp.Permit.Token,
			Amount:      convertBigInt(resp.Permit.Amount),
			Expiration:  resp.Permit.Expiration,
			Nonce:       resp.Permit.Nonce,
			Spender:     resp.Permit.Spender,
			SigDeadline: resp.Permit.SigDeadline,
		}
	}

	if resp.FullCalldata != nil {
		result.FullCalldata = resp.FullCalldata
	}

	return result, nil
}

// solutionToProto converts a Go Solution to a protobuf Solution.
func solutionToProto(s *Solution) *pb.Solution {
	if s == nil {
		return nil
	}

	pbSolution := &pb.Solution{
		GivenToken:    s.GivenToken,
		CheckedToken:  s.CheckedToken,
		Sender:        s.Sender,
		Receiver:      s.Receiver,
		ExactOut:      s.ExactOut,
		RouterAddress: s.RouterAddress,
	}

	if s.GivenAmount != nil {
		pbSolution.GivenAmount = &pb.BigInt{Value: s.GivenAmount.Value}
	}

	if s.CheckedAmount != nil {
		pbSolution.CheckedAmount = &pb.BigInt{Value: s.CheckedAmount.Value}
	}

	if s.NativeAction != nil {
		pbSolution.NativeAction = (*pb.NativeAction)(s.NativeAction)
	}

	if len(s.Swaps) > 0 {
		pbSolution.Swaps = make([]*pb.Swap, len(s.Swaps))
		for i, swap := range s.Swaps {
			pbSolution.Swaps[i] = swapToProto(swap)
		}
	}

	return pbSolution
}

// swapToProto converts a Go Swap to a protobuf Swap.
func swapToProto(s *Swap) *pb.Swap {
	if s == nil {
		return nil
	}

	pbSwap := &pb.Swap{
		TokenIn:  s.TokenIn,
		TokenOut: s.TokenOut,
		Split:    s.Split,
	}

	if s.Component != nil {
		pbSwap.Component = &pb.ProtocolComponent{
			Id:                s.Component.ID,
			ProtocolSystem:    s.Component.ProtocolSystem,
			ProtocolTypeName:  s.Component.ProtocolTypeName,
			Chain:             chainToProto(s.Component.Chain),
			Tokens:            s.Component.Tokens,
			ContractAddresses: s.Component.ContractAddresses,
			StaticAttributes:  s.Component.StaticAttributes,
			CreationTx:        s.Component.CreationTx,
		}
	}

	if len(s.ProtocolState) > 0 {
		pbSwap.ProtocolState = s.ProtocolState
	}

	if s.EstimatedAmountIn != nil {
		pbSwap.EstimatedAmountIn = &pb.BigInt{Value: s.EstimatedAmountIn.Value}
	}

	return pbSwap
}
