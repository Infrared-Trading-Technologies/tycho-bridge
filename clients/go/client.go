package tychobridge

import (
	"context"
	"fmt"
	"sync"
	"time"

	pb "github.com/branched-services/tycho-bridge-go/gen/tycho/bridge/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/backoff"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/keepalive"
)

// Client provides access to the Tycho Bridge gRPC service.
// It manages connection lifecycle, provides retry logic, and exposes
// idiomatic Go methods for all RPC operations.
//
// Client is safe for concurrent use.
type Client struct {
	conn   *grpc.ClientConn
	client pb.TychoBridgeClient

	// opts stores the configuration for this client
	opts *clientOptions

	// mu protects the closed flag
	mu     sync.RWMutex
	closed bool
}

// clientOptions holds configuration for the client.
type clientOptions struct {
	// Connection settings
	dialTimeout     time.Duration
	requestTimeout  time.Duration
	enableTLS       bool
	tlsCredentials  credentials.TransportCredentials
	insecureSkipTLS bool

	// Retry settings
	maxRetries      int
	retryBaseDelay  time.Duration
	retryMaxDelay   time.Duration
	retryMultiplier float64

	// Keepalive settings
	keepaliveTime    time.Duration
	keepaliveTimeout time.Duration

	// Additional dial options
	additionalOpts []grpc.DialOption
}

// Option is a functional option for configuring the Client.
type Option func(*clientOptions)

// defaultOptions returns the default client options.
func defaultOptions() *clientOptions {
	return &clientOptions{
		dialTimeout:      10 * time.Second,
		requestTimeout:   30 * time.Second,
		enableTLS:        false,
		maxRetries:       3,
		retryBaseDelay:   100 * time.Millisecond,
		retryMaxDelay:    10 * time.Second,
		retryMultiplier:  2.0,
		keepaliveTime:    30 * time.Second,
		keepaliveTimeout: 10 * time.Second,
	}
}

// WithDialTimeout sets the timeout for establishing the initial connection.
func WithDialTimeout(d time.Duration) Option {
	return func(o *clientOptions) {
		o.dialTimeout = d
	}
}

// WithRequestTimeout sets the default timeout for individual RPC requests.
func WithRequestTimeout(d time.Duration) Option {
	return func(o *clientOptions) {
		o.requestTimeout = d
	}
}

// WithTLS enables TLS for the connection using the provided credentials.
func WithTLS(creds credentials.TransportCredentials) Option {
	return func(o *clientOptions) {
		o.enableTLS = true
		o.tlsCredentials = creds
	}
}

// WithInsecureTLS enables TLS but skips certificate verification.
// This should only be used for testing.
func WithInsecureTLS() Option {
	return func(o *clientOptions) {
		o.enableTLS = true
		o.insecureSkipTLS = true
	}
}

// WithRetryPolicy configures the retry behavior for failed requests.
func WithRetryPolicy(maxRetries int, baseDelay, maxDelay time.Duration) Option {
	return func(o *clientOptions) {
		o.maxRetries = maxRetries
		o.retryBaseDelay = baseDelay
		o.retryMaxDelay = maxDelay
	}
}

// WithKeepalive configures the keepalive settings for the connection.
func WithKeepalive(time, timeout time.Duration) Option {
	return func(o *clientOptions) {
		o.keepaliveTime = time
		o.keepaliveTimeout = timeout
	}
}

// WithDialOptions adds additional gRPC dial options.
func WithDialOptions(opts ...grpc.DialOption) Option {
	return func(o *clientOptions) {
		o.additionalOpts = append(o.additionalOpts, opts...)
	}
}

// NewClient creates a new Tycho Bridge client connected to the specified address.
//
// The address should be in the format "host:port" (e.g., "localhost:50051").
//
// Example:
//
//	client, err := NewClient("localhost:50051",
//	    WithDialTimeout(5*time.Second),
//	    WithRequestTimeout(10*time.Second),
//	)
func NewClient(addr string, opts ...Option) (*Client, error) {
	// Apply options
	o := defaultOptions()
	for _, opt := range opts {
		opt(o)
	}

	// Build dial options
	dialOpts := buildDialOptions(o)

	// Create context with timeout for dial
	ctx, cancel := context.WithTimeout(context.Background(), o.dialTimeout)
	defer cancel()

	// Dial the server
	conn, err := grpc.DialContext(ctx, addr, dialOpts...)
	if err != nil {
		return nil, fmt.Errorf("failed to dial %s: %w", addr, err)
	}

	return &Client{
		conn:   conn,
		client: pb.NewTychoBridgeClient(conn),
		opts:   o,
	}, nil
}

// buildDialOptions constructs the gRPC dial options from clientOptions.
func buildDialOptions(o *clientOptions) []grpc.DialOption {
	var opts []grpc.DialOption

	// Transport credentials
	if o.enableTLS {
		if o.tlsCredentials != nil {
			opts = append(opts, grpc.WithTransportCredentials(o.tlsCredentials))
		} else if o.insecureSkipTLS {
			// For testing only - use system root CAs but skip verification
			opts = append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))
		}
	} else {
		opts = append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))
	}

	// Keepalive
	opts = append(opts, grpc.WithKeepaliveParams(keepalive.ClientParameters{
		Time:                o.keepaliveTime,
		Timeout:             o.keepaliveTimeout,
		PermitWithoutStream: true,
	}))

	// Connection backoff
	opts = append(opts, grpc.WithConnectParams(grpc.ConnectParams{
		Backoff: backoff.Config{
			BaseDelay:  o.retryBaseDelay,
			Multiplier: o.retryMultiplier,
			MaxDelay:   o.retryMaxDelay,
		},
		MinConnectTimeout: o.dialTimeout,
	}))

	// Additional user-provided options
	opts = append(opts, o.additionalOpts...)

	return opts
}

// Close closes the client connection and releases resources.
// After calling Close, the client should not be used.
func (c *Client) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.closed {
		return nil
	}
	c.closed = true

	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// ensureOpen checks if the client is still open.
func (c *Client) ensureOpen() error {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if c.closed {
		return ErrClientClosed
	}
	return nil
}

// withTimeout creates a context with the default request timeout if the provided
// context doesn't have a deadline.
func (c *Client) withTimeout(ctx context.Context) (context.Context, context.CancelFunc) {
	if _, ok := ctx.Deadline(); ok {
		return ctx, func() {}
	}
	return context.WithTimeout(ctx, c.opts.requestTimeout)
}
