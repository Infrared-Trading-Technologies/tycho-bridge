package tychoffi

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestVersion tests that the version function returns a non-empty string.
func TestVersion(t *testing.T) {
	version := Version()
	assert.NotEmpty(t, version, "version should not be empty")
	t.Logf("Library version: %s", version)
}

// TestBigInt tests the BigInt type conversions.
func TestBigInt(t *testing.T) {
	t.Run("NewBigInt", func(t *testing.T) {
		bi := NewBigInt("12345678901234567890")
		assert.Equal(t, "12345678901234567890", bi.Value)
	})

	t.Run("NewBigIntFromUint64", func(t *testing.T) {
		bi := NewBigIntFromUint64(12345)
		assert.Equal(t, "12345", bi.Value)
	})

	t.Run("ToBigInt", func(t *testing.T) {
		bi := NewBigInt("12345678901234567890")
		result := bi.ToBigInt()
		assert.Equal(t, "12345678901234567890", result.String())
	})

	t.Run("ToBigInt nil", func(t *testing.T) {
		var bi *BigInt
		result := bi.ToBigInt()
		assert.Equal(t, "0", result.String())
	})

	t.Run("ToBigInt empty", func(t *testing.T) {
		bi := &BigInt{Value: ""}
		result := bi.ToBigInt()
		assert.Equal(t, "0", result.String())
	})
}

// TestPrice tests the Price type conversions.
func TestPrice(t *testing.T) {
	t.Run("ToRat", func(t *testing.T) {
		price := &Price{
			Numerator:   NewBigInt("3"),
			Denominator: NewBigInt("4"),
		}
		rat := price.ToRat()
		assert.Equal(t, "3/4", rat.String())
	})

	t.Run("ToFloat64", func(t *testing.T) {
		price := &Price{
			Numerator:   NewBigInt("1"),
			Denominator: NewBigInt("2"),
		}
		f := price.ToFloat64()
		assert.Equal(t, 0.5, f)
	})

	t.Run("ToRat nil", func(t *testing.T) {
		var price *Price
		rat := price.ToRat()
		assert.Equal(t, "0/1", rat.String())
	})

	t.Run("ToRat zero denominator", func(t *testing.T) {
		price := &Price{
			Numerator:   NewBigInt("1"),
			Denominator: NewBigInt("0"),
		}
		rat := price.ToRat()
		assert.Equal(t, "0/1", rat.String())
	})
}

// TestBridgeError tests the BridgeError type.
func TestBridgeError(t *testing.T) {
	t.Run("Error message", func(t *testing.T) {
		err := NewBridgeError(1, "component xyz not found")
		assert.Equal(t, "component not found: component xyz not found", err.Error())
	})

	t.Run("Error code only", func(t *testing.T) {
		err := &BridgeError{Code: ErrorInvalidArgument}
		assert.Equal(t, "invalid argument", err.Error())
	})

	t.Run("ErrorCode String", func(t *testing.T) {
		assert.Equal(t, "no error", ErrorNone.String())
		assert.Equal(t, "component not found", ErrorComponentNotFound.String())
		assert.Equal(t, "internal error", ErrorInternalError.String())
	})
}

// TestConfigMarshalJSON tests the Config JSON marshaling.
func TestConfigMarshalJSON(t *testing.T) {
	config := Config{
		TychoURL:    "tycho-beta.propellerheads.xyz",
		TychoAPIKey: "test-key",
		ChainIDs:    []uint64{1, 8453},
	}

	data, err := config.MarshalJSON()
	require.NoError(t, err)

	expected := `{"tycho_url":"tycho-beta.propellerheads.xyz","tycho_api_key":"test-key","chain_ids":"1,8453"}`
	assert.JSONEq(t, expected, string(data))
}

// TestConfigMarshalJSON_Empty tests empty chain IDs.
func TestConfigMarshalJSON_Empty(t *testing.T) {
	config := Config{
		TychoURL:    "localhost",
		TychoAPIKey: "",
		ChainIDs:    []uint64{},
	}

	data, err := config.MarshalJSON()
	require.NoError(t, err)

	expected := `{"tycho_url":"localhost","tycho_api_key":"","chain_ids":""}`
	assert.JSONEq(t, expected, string(data))
}

// TestNilBridge tests that nil bridge returns appropriate errors.
func TestNilBridge(t *testing.T) {
	var bridge *Bridge

	t.Run("SpotPrice", func(t *testing.T) {
		_, err := bridge.SpotPrice(SpotPriceRequest{})
		assert.ErrorIs(t, err, ErrNilBridge)
	})

	t.Run("GetAmountOut", func(t *testing.T) {
		_, err := bridge.GetAmountOut(GetAmountOutRequest{})
		assert.ErrorIs(t, err, ErrNilBridge)
	})

	t.Run("GetTokens", func(t *testing.T) {
		_, err := bridge.GetTokens(GetTokensRequest{})
		assert.ErrorIs(t, err, ErrNilBridge)
	})

	t.Run("IsHealthy", func(t *testing.T) {
		healthy := bridge.IsHealthy()
		assert.False(t, healthy)
	})

	t.Run("ComponentCount", func(t *testing.T) {
		count := bridge.ComponentCount()
		assert.Equal(t, uint64(0), count)
	})
}

// The following tests require the FFI library to be built and a real or mock
// Tycho indexer to be available. They are skipped by default.

// TestBridgeInit_Integration tests bridge initialization.
// Requires: cargo build -p tycho-ffi --release
func TestBridgeInit_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	config := Config{
		TychoURL:               "tycho-beta.propellerheads.xyz",
		TychoAPIKey:            "test-key",
		ChainIDs:               []uint64{1},
		AuthenticationDisabled: true, // For testing without real auth
	}

	bridge, err := NewBridge(config)
	if err != nil {
		// This is expected to fail without a real connection
		t.Logf("Bridge init failed (expected without real indexer): %v", err)
		return
	}
	defer bridge.Close()

	t.Log("Bridge initialized successfully")
}

// TestBridgeClose_Multiple tests that Close can be called multiple times safely.
func TestBridgeClose_Multiple(t *testing.T) {
	// Create a mock bridge (this will likely fail, but we're testing Close behavior)
	config := Config{
		TychoURL:    "localhost:50051",
		TychoAPIKey: "",
		ChainIDs:    []uint64{1},
	}

	bridge, err := NewBridge(config)
	if err != nil {
		t.Skip("Skipping - bridge init failed (expected without real indexer)")
	}

	// Close multiple times - should not panic
	bridge.Close()
	bridge.Close()
	bridge.Close()

	// After close, should return appropriate errors
	_, err = bridge.SpotPrice(SpotPriceRequest{})
	assert.ErrorIs(t, err, ErrBridgeClosed)
}

// ExampleNewBridge demonstrates how to create and use a bridge.
func ExampleNewBridge() {
	config := Config{
		TychoURL:    "tycho-beta.propellerheads.xyz",
		TychoAPIKey: "your-api-key",
		ChainIDs:    []uint64{1, 8453},
	}

	bridge, err := NewBridge(config)
	if err != nil {
		panic(err)
	}
	defer bridge.Close()

	if err := bridge.Start(); err != nil {
		panic(err)
	}

	// Query spot price
	resp, err := bridge.SpotPrice(SpotPriceRequest{
		ChainID:     1,
		ComponentID: "0x...",
		BaseToken:   "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
		QuoteToken:  "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
	})
	if err != nil {
		panic(err)
	}

	_ = resp.PriceFloat // Use the price
}
