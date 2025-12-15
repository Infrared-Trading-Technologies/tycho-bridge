package tychobridge

import (
	"context"
	"fmt"
	"io"
	"sync"
	"time"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
)

// UpdateType represents the type of update.
type UpdateType int

const (
	// UpdateTypeUnspecified is the default unknown type.
	UpdateTypeUnspecified UpdateType = iota

	// UpdateTypeNew indicates a new component was added.
	UpdateTypeNew

	// UpdateTypeModified indicates a component state changed.
	UpdateTypeModified

	// UpdateTypeRemoved indicates a component was removed.
	UpdateTypeRemoved
)

// String returns the string representation of the update type.
func (t UpdateType) String() string {
	switch t {
	case UpdateTypeNew:
		return "new"
	case UpdateTypeModified:
		return "modified"
	case UpdateTypeRemoved:
		return "removed"
	default:
		return "unspecified"
	}
}

// ComponentState contains dynamic attributes that change per block.
type ComponentState struct {
	// ComponentID is the component this state belongs to.
	ComponentID string

	// Attributes are dynamic state attributes (reserves, balances, tick, etc.).
	Attributes map[string][]byte

	// Balances are component balances per token address.
	Balances map[string]*BigInt

	// TVL is the estimated TVL in native token.
	TVL float64
}

// ContractState contains contract storage for VM simulations.
type ContractState struct {
	// Address is the contract address.
	Address string

	// Code is the contract bytecode.
	Code []byte

	// Storage is the storage slots (key -> value).
	Storage map[string][]byte

	// Balance is the native balance of the contract.
	Balance *BigInt
}

// Update represents a streaming update message.
type Update struct {
	// BlockNumber is the block number of this update.
	BlockNumber uint64

	// BlockHash is the block hash.
	BlockHash string

	// Timestamp is the block timestamp.
	Timestamp uint64

	// NewComponents are newly created components.
	NewComponents []*ProtocolComponent

	// RemovedComponentIDs are components no longer tracked.
	RemovedComponentIDs []string

	// StateUpdates are updated states for modified components.
	StateUpdates []*ComponentState

	// ContractUpdates are contract storage changes for VM simulations.
	ContractUpdates []*ContractState

	// NewTokens are newly discovered tokens.
	NewTokens []*Token
}

// StreamUpdatesRequest contains the parameters for streaming updates.
type StreamUpdatesRequest struct {
	// Chain to stream from (required).
	Chain Chain

	// ProtocolSystems filters by protocol systems.
	ProtocolSystems []string

	// MinTVL is the minimum TVL threshold for automatic tracking.
	MinTVL *float64

	// AddTVLThreshold is the TVL threshold for adding components.
	AddTVLThreshold *float64

	// RemoveTVLThreshold is the TVL threshold for removing components.
	RemoveTVLThreshold *float64

	// ComponentIDs filters by specific component IDs.
	ComponentIDs []string

	// IncludeContractState includes contract storage for VM simulations.
	IncludeContractState bool
}

// UpdateStream provides a channel-based interface for receiving updates.
type UpdateStream struct {
	updates <-chan *Update
	errors  <-chan error
	cancel  context.CancelFunc

	mu     sync.RWMutex
	closed bool
}

// Updates returns the channel for receiving updates.
func (s *UpdateStream) Updates() <-chan *Update {
	return s.updates
}

// Errors returns the channel for receiving stream errors.
func (s *UpdateStream) Errors() <-chan error {
	return s.errors
}

// Close closes the stream and releases resources.
func (s *UpdateStream) Close() {
	s.mu.Lock()
	defer s.mu.Unlock()

	if !s.closed {
		s.closed = true
		s.cancel()
	}
}

// IsClosed returns true if the stream has been closed.
func (s *UpdateStream) IsClosed() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.closed
}

// buildStreamUpdatesRequest builds a protobuf StreamUpdatesRequest from the Go request type.
func buildStreamUpdatesRequest(req *StreamUpdatesRequest) *pb.StreamUpdatesRequest {
	pbReq := &pb.StreamUpdatesRequest{
		Chain:                chainToProto(req.Chain),
		ProtocolSystems:      req.ProtocolSystems,
		ComponentIds:         req.ComponentIDs,
		IncludeContractState: req.IncludeContractState,
	}

	if req.MinTVL != nil {
		pbReq.MinTvl = req.MinTVL
	}
	if req.AddTVLThreshold != nil {
		pbReq.AddTvlThreshold = req.AddTVLThreshold
	}
	if req.RemoveTVLThreshold != nil {
		pbReq.RemoveTvlThreshold = req.RemoveTVLThreshold
	}

	return pbReq
}

// StreamUpdates subscribes to real-time protocol state changes.
// Returns an UpdateStream that provides channels for updates and errors.
//
// The returned stream should be closed when no longer needed.
//
// Example:
//
//	stream, err := client.StreamUpdates(ctx, &tychobridge.StreamUpdatesRequest{
//	    Chain: tychobridge.ChainEthereum,
//	})
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer stream.Close()
//
//	for {
//	    select {
//	    case update := <-stream.Updates():
//	        fmt.Printf("Block %d received\n", update.BlockNumber)
//	    case err := <-stream.Errors():
//	        log.Printf("Stream error: %v\n", err)
//	    }
//	}
func (c *Client) StreamUpdates(ctx context.Context, req *StreamUpdatesRequest) (*UpdateStream, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	// Create a cancellable context for the stream
	streamCtx, cancel := context.WithCancel(ctx)

	pbReq := buildStreamUpdatesRequest(req)

	stream, err := c.client.StreamUpdates(streamCtx, pbReq)
	if err != nil {
		cancel()
		return nil, fmt.Errorf("StreamUpdates failed: %w", err)
	}

	// Create buffered channels
	updatesChan := make(chan *Update, 100)
	errorsChan := make(chan error, 1)

	updateStream := &UpdateStream{
		updates: updatesChan,
		errors:  errorsChan,
		cancel:  cancel,
	}

	// Start a goroutine to receive updates
	go func() {
		defer close(updatesChan)
		defer close(errorsChan)

		for {
			pbUpdate, err := stream.Recv()
			if err != nil {
				if err == io.EOF {
					return
				}
				if streamCtx.Err() != nil {
					// Context was cancelled, normal shutdown
					return
				}
				select {
				case errorsChan <- fmt.Errorf("stream receive error: %w", err):
				default:
				}
				return
			}

			update := convertUpdate(pbUpdate)
			select {
			case updatesChan <- update:
			case <-streamCtx.Done():
				return
			}
		}
	}()

	return updateStream, nil
}

// StreamUpdatesWithReconnect is like StreamUpdates but automatically
// reconnects on stream errors with exponential backoff.
func (c *Client) StreamUpdatesWithReconnect(ctx context.Context, req *StreamUpdatesRequest) (*UpdateStream, error) {
	if err := c.ensureOpen(); err != nil {
		return nil, err
	}

	streamCtx, cancel := context.WithCancel(ctx)

	updatesChan := make(chan *Update, 100)
	errorsChan := make(chan error, 10)

	updateStream := &UpdateStream{
		updates: updatesChan,
		errors:  errorsChan,
		cancel:  cancel,
	}

	// Start reconnection manager
	go c.manageUpdateStreamReconnection(streamCtx, req, updatesChan, errorsChan)

	return updateStream, nil
}

// manageUpdateStreamReconnection handles automatic reconnection with exponential backoff.
func (c *Client) manageUpdateStreamReconnection(
	ctx context.Context,
	req *StreamUpdatesRequest,
	updatesChan chan<- *Update,
	errorsChan chan<- error,
) {
	defer close(updatesChan)
	defer close(errorsChan)

	backoffDuration := c.opts.retryBaseDelay
	maxBackoff := c.opts.retryMaxDelay

	for {
		select {
		case <-ctx.Done():
			return
		default:
		}

		err := c.runUpdateStream(ctx, req, updatesChan)
		if err == nil {
			return
		}

		if ctx.Err() != nil {
			return
		}

		// Send error to errors channel
		select {
		case errorsChan <- err:
		default:
		}

		// Exponential backoff
		select {
		case <-time.After(backoffDuration):
		case <-ctx.Done():
			return
		}

		backoffDuration = time.Duration(float64(backoffDuration) * c.opts.retryMultiplier)
		if backoffDuration > maxBackoff {
			backoffDuration = maxBackoff
		}
	}
}

// runUpdateStream runs a single stream connection until error or context cancellation.
func (c *Client) runUpdateStream(
	ctx context.Context,
	req *StreamUpdatesRequest,
	updatesChan chan<- *Update,
) error {
	pbReq := buildStreamUpdatesRequest(req)

	stream, err := c.client.StreamUpdates(ctx, pbReq)
	if err != nil {
		return fmt.Errorf("failed to start stream: %w", err)
	}

	for {
		pbUpdate, err := stream.Recv()
		if err != nil {
			if err == io.EOF || ctx.Err() != nil {
				return nil
			}
			return fmt.Errorf("stream receive error: %w", err)
		}

		update := convertUpdate(pbUpdate)
		select {
		case updatesChan <- update:
		case <-ctx.Done():
			return nil
		}
	}
}

// convertUpdate converts a protobuf Update to a Go Update.
func convertUpdate(u *pb.Update) *Update {
	if u == nil {
		return nil
	}

	return &Update{
		BlockNumber:         u.BlockNumber,
		BlockHash:           u.BlockHash,
		Timestamp:           u.Timestamp,
		NewComponents:       convertProtocolComponents(u.NewComponents),
		RemovedComponentIDs: u.RemovedComponentIds,
		StateUpdates:        convertComponentStates(u.StateUpdates),
		ContractUpdates:     convertContractStates(u.ContractUpdates),
		NewTokens:           convertTokens(u.NewTokens),
	}
}

// convertComponentStates converts protobuf ComponentState messages to Go.
func convertComponentStates(pbStates []*pb.ComponentState) []*ComponentState {
	states := make([]*ComponentState, len(pbStates))
	for i, s := range pbStates {
		states[i] = convertComponentState(s)
	}
	return states
}

// convertComponentState converts a protobuf ComponentState to Go.
func convertComponentState(s *pb.ComponentState) *ComponentState {
	if s == nil {
		return nil
	}

	balances := make(map[string]*BigInt)
	for k, v := range s.Balances {
		balances[k] = convertBigInt(v)
	}

	return &ComponentState{
		ComponentID: s.ComponentId,
		Attributes:  s.Attributes,
		Balances:    balances,
		TVL:         s.Tvl,
	}
}

// convertContractStates converts protobuf ContractState messages to Go.
func convertContractStates(pbStates []*pb.ContractState) []*ContractState {
	states := make([]*ContractState, len(pbStates))
	for i, s := range pbStates {
		states[i] = convertContractState(s)
	}
	return states
}

// convertContractState converts a protobuf ContractState to Go.
func convertContractState(s *pb.ContractState) *ContractState {
	if s == nil {
		return nil
	}

	return &ContractState{
		Address: s.Address,
		Code:    s.Code,
		Storage: s.Storage,
		Balance: convertBigInt(s.Balance),
	}
}
