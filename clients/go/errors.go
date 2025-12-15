package tychobridge

import "errors"

// Error definitions for the Tycho Bridge client.
var (
	// ErrClientClosed is returned when an operation is attempted on a closed client.
	ErrClientClosed = errors.New("tychobridge: client is closed")

	// ErrNilResponse is returned when the server returns a nil response.
	ErrNilResponse = errors.New("tychobridge: server returned nil response")
)
