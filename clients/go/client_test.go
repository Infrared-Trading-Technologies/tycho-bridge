package tychobridge

import (
	"context"
	"testing"
	"time"
)

func TestDefaultOptions(t *testing.T) {
	opts := defaultOptions()

	if opts.dialTimeout != 10*time.Second {
		t.Errorf("expected dialTimeout 10s, got %v", opts.dialTimeout)
	}
	if opts.requestTimeout != 30*time.Second {
		t.Errorf("expected requestTimeout 30s, got %v", opts.requestTimeout)
	}
	if opts.maxRetries != 3 {
		t.Errorf("expected maxRetries 3, got %v", opts.maxRetries)
	}
	if opts.retryBaseDelay != 100*time.Millisecond {
		t.Errorf("expected retryBaseDelay 100ms, got %v", opts.retryBaseDelay)
	}
	if opts.retryMaxDelay != 10*time.Second {
		t.Errorf("expected retryMaxDelay 10s, got %v", opts.retryMaxDelay)
	}
	if opts.enableTLS {
		t.Error("expected TLS disabled by default")
	}
}

func TestWithDialTimeout(t *testing.T) {
	opts := defaultOptions()
	WithDialTimeout(5 * time.Second)(opts)

	if opts.dialTimeout != 5*time.Second {
		t.Errorf("expected dialTimeout 5s, got %v", opts.dialTimeout)
	}
}

func TestWithRequestTimeout(t *testing.T) {
	opts := defaultOptions()
	WithRequestTimeout(15 * time.Second)(opts)

	if opts.requestTimeout != 15*time.Second {
		t.Errorf("expected requestTimeout 15s, got %v", opts.requestTimeout)
	}
}

func TestWithRetryPolicy(t *testing.T) {
	opts := defaultOptions()
	WithRetryPolicy(5, 200*time.Millisecond, 30*time.Second)(opts)

	if opts.maxRetries != 5 {
		t.Errorf("expected maxRetries 5, got %v", opts.maxRetries)
	}
	if opts.retryBaseDelay != 200*time.Millisecond {
		t.Errorf("expected retryBaseDelay 200ms, got %v", opts.retryBaseDelay)
	}
	if opts.retryMaxDelay != 30*time.Second {
		t.Errorf("expected retryMaxDelay 30s, got %v", opts.retryMaxDelay)
	}
}

func TestWithKeepalive(t *testing.T) {
	opts := defaultOptions()
	WithKeepalive(60*time.Second, 20*time.Second)(opts)

	if opts.keepaliveTime != 60*time.Second {
		t.Errorf("expected keepaliveTime 60s, got %v", opts.keepaliveTime)
	}
	if opts.keepaliveTimeout != 20*time.Second {
		t.Errorf("expected keepaliveTimeout 20s, got %v", opts.keepaliveTimeout)
	}
}

func TestWithInsecureTLS(t *testing.T) {
	opts := defaultOptions()
	WithInsecureTLS()(opts)

	if !opts.enableTLS {
		t.Error("expected TLS enabled")
	}
	if !opts.insecureSkipTLS {
		t.Error("expected insecureSkipTLS true")
	}
}

func TestClientClose(t *testing.T) {
	// Test that closing a nil client doesn't panic
	c := &Client{closed: false}
	err := c.Close()
	if err != nil {
		t.Errorf("expected nil error, got %v", err)
	}

	// Verify closed flag is set
	if !c.closed {
		t.Error("expected closed to be true after Close()")
	}

	// Double close should be safe
	err = c.Close()
	if err != nil {
		t.Errorf("expected nil error on double close, got %v", err)
	}
}

func TestEnsureOpen(t *testing.T) {
	c := &Client{closed: false}

	err := c.ensureOpen()
	if err != nil {
		t.Errorf("expected nil error for open client, got %v", err)
	}

	c.closed = true
	err = c.ensureOpen()
	if err != ErrClientClosed {
		t.Errorf("expected ErrClientClosed, got %v", err)
	}
}

func TestWithTimeout(t *testing.T) {
	opts := defaultOptions()
	opts.requestTimeout = 5 * time.Second
	c := &Client{opts: opts}

	// Context without deadline should get timeout
	ctx := context.Background()
	newCtx, cancel := c.withTimeout(ctx)
	defer cancel()

	deadline, ok := newCtx.Deadline()
	if !ok {
		t.Error("expected context to have deadline")
	}
	if time.Until(deadline) > 5*time.Second {
		t.Error("deadline should be within request timeout")
	}

	// Context with existing deadline should be unchanged
	existingDeadline := time.Now().Add(10 * time.Second)
	ctxWithDeadline, cancelDeadline := context.WithDeadline(context.Background(), existingDeadline)
	defer cancelDeadline()

	newCtx2, cancel2 := c.withTimeout(ctxWithDeadline)
	defer cancel2()

	deadline2, ok := newCtx2.Deadline()
	if !ok {
		t.Error("expected context to have deadline")
	}
	if !deadline2.Equal(existingDeadline) {
		t.Error("existing deadline should be preserved")
	}
}

func TestUpdateStreamClose(t *testing.T) {
	cancelled := false
	stream := &UpdateStream{
		cancel: func() { cancelled = true },
	}

	if stream.IsClosed() {
		t.Error("stream should not be closed initially")
	}

	stream.Close()

	if !stream.IsClosed() {
		t.Error("stream should be closed after Close()")
	}
	if !cancelled {
		t.Error("cancel function should have been called")
	}

	// Double close should be safe
	stream.Close()
}
