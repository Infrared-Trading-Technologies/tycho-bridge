package tychoffi

import "math/big"

// BigInt represents an unsigned 256-bit integer as a decimal string.
// This is the same representation used by the gRPC client for compatibility.
type BigInt struct {
	Value string
}

// NewBigInt creates a new BigInt from a string value.
func NewBigInt(value string) *BigInt {
	return &BigInt{Value: value}
}

// NewBigIntFromUint64 creates a new BigInt from a uint64 value.
func NewBigIntFromUint64(value uint64) *BigInt {
	return &BigInt{Value: big.NewInt(int64(value)).String()}
}

// NewBigIntFromBigInt creates a new BigInt from a *big.Int.
func NewBigIntFromBigInt(value *big.Int) *BigInt {
	if value == nil {
		return &BigInt{Value: "0"}
	}
	return &BigInt{Value: value.String()}
}

// ToBigInt converts to a *big.Int.
func (b *BigInt) ToBigInt() *big.Int {
	if b == nil || b.Value == "" {
		return big.NewInt(0)
	}
	result, ok := new(big.Int).SetString(b.Value, 10)
	if !ok {
		return big.NewInt(0)
	}
	return result
}

// Price represents a price as a rational fraction.
type Price struct {
	Numerator   *BigInt
	Denominator *BigInt
}

// ToRat converts the price to a *big.Rat.
func (p *Price) ToRat() *big.Rat {
	if p == nil || p.Numerator == nil || p.Denominator == nil {
		return new(big.Rat)
	}
	num := p.Numerator.ToBigInt()
	den := p.Denominator.ToBigInt()
	if den.Sign() == 0 {
		return new(big.Rat)
	}
	return new(big.Rat).SetFrac(num, den)
}

// ToFloat64 converts the price to a float64.
func (p *Price) ToFloat64() float64 {
	rat := p.ToRat()
	f, _ := rat.Float64()
	return f
}

// Chain represents a supported blockchain network.
type Chain uint64

const (
	// ChainEthereum is Ethereum mainnet (chain ID 1).
	ChainEthereum Chain = 1

	// ChainBase is Base (chain ID 8453).
	ChainBase Chain = 8453

	// ChainUnichain is Unichain (chain ID 130).
	ChainUnichain Chain = 130

	// ChainArbitrum is Arbitrum One (chain ID 42161).
	ChainArbitrum Chain = 42161
)

// Token represents an ERC-20 token.
type Token struct {
	// Address is the hex-encoded address (e.g., "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").
	Address string `json:"address"`

	// Symbol is the human-readable symbol (e.g., "WETH").
	Symbol string `json:"symbol"`

	// Decimals is the token decimals (e.g., 18).
	Decimals uint32 `json:"decimals"`

	// Quality is the token quality score (0-100).
	// 100: Normal token, 75: Rebase token, 50: Fee token
	Quality int32 `json:"quality,omitempty"`
}

// Component represents a protocol component (e.g., a liquidity pool).
type Component struct {
	// ID is the unique component identifier.
	ID string `json:"id"`

	// ProtocolSystem is the protocol system (e.g., "uniswap_v2").
	ProtocolSystem string `json:"protocol_system"`

	// ProtocolType is the type of component (e.g., "vm:amm").
	ProtocolType string `json:"protocol_type,omitempty"`

	// Tokens is the list of token addresses in the component.
	Tokens []string `json:"tokens"`

	// TVL is the total value locked in USD.
	TVL float64 `json:"tvl,omitempty"`

	// ChainID is the chain ID this component is on.
	ChainID uint64 `json:"chain_id"`
}

// IndexerStatus represents the indexer sync status.
type IndexerStatus struct {
	// SyncStatus is the current sync status (e.g., "synced", "syncing").
	SyncStatus string `json:"sync_status"`

	// LatestBlock is the latest block number processed.
	LatestBlock uint64 `json:"latest_block"`

	// ComponentCount is the number of components cached.
	ComponentCount uint64 `json:"component_count"`

	// UptimeSeconds is the uptime in seconds.
	UptimeSeconds uint64 `json:"uptime_seconds"`
}

// HealthStatus represents the health check response.
type HealthStatus struct {
	// Healthy is true if the bridge is healthy.
	Healthy bool `json:"healthy"`

	// Message is an optional status message.
	Message string `json:"message,omitempty"`
}
