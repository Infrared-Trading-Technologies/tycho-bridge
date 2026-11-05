//! Domain types for Tycho Bridge.
//!
//! This module re-exports the authoritative Tycho SDK types as the domain primitives.
//! **CONSTRAINT:** Do NOT mock these types. All simulation and state management must
//! use the real Tycho crate implementations.
//!
//! # Architecture
//!
//! The domain layer establishes a clear boundary between:
//! - **Tycho SDK types** (external, authoritative) - used for blockchain state and simulation
//! - **gRPC types** (internal, generated) - used for API transport
//!
//! Mapping between these layers happens at the service boundary, never within core logic.
//!
//! # Crate Version Note
//!
//! `tycho-simulation` re-exports `tycho_client` and `tycho_common` internally.
//! To avoid version conflicts, we ONLY use re-exports from `tycho_simulation`.
//! Do NOT import directly from `tycho-client` crate.

// =============================================================================
// Core Domain Types (via tycho-simulation re-exports for version consistency)
// =============================================================================

/// Chain identifiers for supported blockchains.
/// Used to partition state by chain and route requests correctly.
pub use tycho_simulation::tycho_common::dto::Chain;

/// Response data for contract accounts (code, storage, balance).
/// Required for VM-based simulations (Balancer, Curve, etc.).
pub use tycho_simulation::tycho_common::dto::ResponseAccount;

/// Token representation with metadata (address, decimals, symbol).
/// Immutable reference data for swap simulation.
pub use tycho_simulation::tycho_common::models::token::Token;

/// Protocol component representing a liquidity pool or similar construct.
/// Contains static protocol information (tokens, fees, etc.).
pub use tycho_simulation::protocol::models::ProtocolComponent;

// =============================================================================
// Client Types (via tycho-simulation re-exports)
// =============================================================================

/// Stream builder for connecting to Tycho indexer.
/// Provides fluent API for configuring exchange filters and authentication.
pub use tycho_simulation::tycho_client::stream::TychoStreamBuilder;

/// Filter for selecting components by TVL or specific IDs.
/// Used to limit the scope of state synchronization.
pub use tycho_simulation::tycho_client::feed::component_tracker::ComponentFilter;

/// HTTP RPC client for Tycho indexer API.
/// Used for fetching tokens, historical state, and other reference data.
pub use tycho_simulation::tycho_client::rpc::HttpRPCClient;

/// HTTP RPC client options for configuration.
pub use tycho_simulation::tycho_client::rpc::HttpRPCClientOptions;

// =============================================================================
// Simulation Types (via tycho-simulation re-exports)
// =============================================================================

/// Core simulation trait implemented by all protocol states.
/// Provides `get_amount_out`, `spot_price`, `fee`, `get_limits`, etc.
pub use tycho_simulation::tycho_common::simulation::protocol_sim::ProtocolSim;

/// Result of a swap simulation containing amount out, gas cost, and new state.
pub use tycho_simulation::tycho_common::simulation::protocol_sim::GetAmountOutResult;

/// Simulation error types for handling edge cases.
pub use tycho_simulation::tycho_common::simulation::errors::SimulationError;

/// Pool swap query types backing swap_to_price and query_supply.
/// `QueryPoolSwapParams` carries the token pair and a `SwapConstraint`
/// (`PoolTargetPrice` or `TradeLimitPrice`); the result is a `PoolSwap`.
pub use tycho_simulation::tycho_common::simulation::protocol_sim::{
    PoolSwap, QueryPoolSwapParams, SwapConstraint,
};

/// Price representation as a rational fraction (numerator/denominator).
pub use tycho_simulation::tycho_common::simulation::protocol_sim::Price;

// =============================================================================
// Protocol-Specific States (for direct instantiation)
// =============================================================================

/// Uniswap V2 pool state for constant-product AMM simulation.
pub use tycho_simulation::evm::protocol::uniswap_v2::state::UniswapV2State;

/// Uniswap V3 pool state for concentrated liquidity simulation.
pub use tycho_simulation::evm::protocol::uniswap_v3::state::UniswapV3State;

/// EVM-based pool state for VM simulations (Balancer, Curve, etc.).
pub use tycho_simulation::evm::protocol::vm::state::EVMPoolState;

// =============================================================================
// Stream Types (for protocol streaming)
// =============================================================================

/// Protocol stream builder for setting up multi-protocol state streams.
pub use tycho_simulation::evm::stream::ProtocolStreamBuilder;

// =============================================================================
// Execution Types (for calldata encoding)
// =============================================================================

/// Solution struct representing a swap route for encoding.
/// Contains sender, receiver, tokens, amounts, and swap steps.
pub use tycho_execution::encoding::models::Solution;

/// Swap struct representing a single swap step within a solution.
pub use tycho_execution::encoding::models::Swap;

/// SwapBuilder for constructing Swap objects with builder pattern.
pub use tycho_execution::encoding::models::SwapBuilder;

/// User transfer type for router encoding (TransferFrom, TransferFromPermit2, etc.)
pub use tycho_execution::encoding::models::UserTransferType;

/// Native action for wrap/unwrap ETH operations.
pub use tycho_execution::encoding::models::NativeAction;

/// EncodedSolution containing calldata and router address.
pub use tycho_execution::encoding::models::EncodedSolution;

/// Transaction struct with to, value, and data for execution.
pub use tycho_execution::encoding::models::Transaction as ExecutionTransaction;

/// TychoRouterEncoderBuilder for configuring the encoder.
pub use tycho_execution::encoding::evm::encoder_builders::TychoRouterEncoderBuilder;

/// TychoEncoder trait for encoding solutions.
pub use tycho_execution::encoding::tycho_encoder::TychoEncoder;

/// Encoding errors.
pub use tycho_execution::encoding::errors::EncodingError;

/// Bytes type for addresses and other hex data.
/// Re-exported from tycho_common for consistency with execution types.
pub use tycho_simulation::tycho_common::Bytes;

// =============================================================================
// Type Aliases for Common Patterns
// =============================================================================

use std::collections::HashMap;

/// Chain-partitioned state cache.
/// Outer key is chain ID, inner value is a DashMap of component ID to ProtocolSim.
/// Note: For most use cases, prefer using the StateCache struct from state_cache module.
pub type ChainStateCache =
    dashmap::DashMap<u64, dashmap::DashMap<String, std::sync::Arc<dyn ProtocolSim + Send + Sync>>>;

/// Token registry for quick lookups by address.
pub type TokenRegistry = HashMap<String, Token>;

/// Chain-partitioned token registry.
pub type ChainTokenRegistry = HashMap<u64, TokenRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that core types are accessible and can be imported.
    /// This test ensures the re-exports are correctly configured.
    #[test]
    fn test_domain_types_accessible() {
        // Chain enum should be usable
        let _chain = Chain::Ethereum;
        let _base = Chain::Base;

        // Verify we can reference the simulation trait
        fn accepts_protocol_sim<T: ProtocolSim>(_: &T) {}
        let _ = accepts_protocol_sim::<UniswapV2State>;
    }

    #[test]
    fn test_component_filter_creation() {
        // TVL range filter
        let _tvl_filter = ComponentFilter::with_tvl_range(10.0, 100.0);

        // Specific component IDs filter
        let _ids_filter = ComponentFilter::Ids(vec![
            "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".to_string()
        ]);
    }
}
