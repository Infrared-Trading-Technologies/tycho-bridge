//! Shared test utilities for tycho-core integration tests.
//!
//! This module follows the idiomatic Rust pattern for shared test utilities.

use std::collections::HashMap;
use std::sync::Arc;

use num_bigint::BigUint;

// Use types from tycho_core for consistency
use tycho_core::domain::{Bytes, Chain, ProtocolComponent, ProtocolSim, Token};
use tycho_core::state::{StateCache, TransientStateCache};
use tycho_core::BridgeService;
use tycho_simulation::tycho_common::dto::ProtocolStateDelta;
use tycho_simulation::tycho_common::simulation::errors::{SimulationError, TransitionError};
use tycho_simulation::tycho_common::simulation::protocol_sim::{Balances, GetAmountOutResult};

/// Mock protocol simulation state for testing.
///
/// Simulates a constant-product AMM (like UniswapV2) with configurable reserves.
#[derive(Clone, Debug)]
pub struct MockProtocolSim {
    pub reserve0: u64,
    pub reserve1: u64,
    pub fee: f64,
}

impl MockProtocolSim {
    pub fn new(reserve0: u64, reserve1: u64) -> Self {
        Self {
            reserve0,
            reserve1,
            fee: 0.003,
        }
    }

    #[allow(dead_code)]
    pub fn with_fee(reserve0: u64, reserve1: u64, fee: f64) -> Self {
        Self {
            reserve0,
            reserve1,
            fee,
        }
    }
}

impl ProtocolSim for MockProtocolSim {
    fn fee(&self) -> f64 {
        self.fee
    }

    fn spot_price(&self, _base: &Token, _quote: &Token) -> Result<f64, SimulationError> {
        Ok(self.reserve1 as f64 / self.reserve0 as f64)
    }

    fn get_amount_out(
        &self,
        amount_in: BigUint,
        _token_in: &Token,
        _token_out: &Token,
    ) -> Result<GetAmountOutResult, SimulationError> {
        let amount_in_u64: u64 = amount_in.try_into().unwrap_or(0);

        if amount_in_u64 == 0 {
            return Err(SimulationError::FatalError("Amount in is zero".to_string()));
        }

        let fee_factor = 1.0 - self.fee;
        let amount_in_with_fee = (amount_in_u64 as f64 * fee_factor) as u64;

        let amount_out = (self.reserve1 as u128 * amount_in_with_fee as u128)
            / (self.reserve0 as u128 + amount_in_with_fee as u128);

        let new_state = Box::new(MockProtocolSim {
            reserve0: self.reserve0 + amount_in_u64,
            reserve1: self.reserve1 - amount_out as u64,
            fee: self.fee,
        });

        Ok(GetAmountOutResult {
            amount: BigUint::from(amount_out),
            gas: BigUint::from(100_000u64),
            new_state,
        })
    }

    fn get_limits(
        &self,
        _sell_token: Bytes,
        _buy_token: Bytes,
    ) -> Result<(BigUint, BigUint), SimulationError> {
        Ok((BigUint::from(self.reserve0), BigUint::from(self.reserve1)))
    }

    fn delta_transition(
        &mut self,
        _delta: ProtocolStateDelta,
        _tokens: &HashMap<Bytes, Token>,
        _balances: &Balances,
    ) -> Result<(), TransitionError<String>> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ProtocolSim> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn eq(&self, other: &dyn ProtocolSim) -> bool {
        other
            .as_any()
            .downcast_ref::<MockProtocolSim>()
            .map(|o| o.reserve0 == self.reserve0 && o.reserve1 == self.reserve1)
            .unwrap_or(false)
    }
}

/// Well-known test addresses.
#[allow(dead_code)]
pub mod addresses {
    pub const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    pub const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    pub const POOL_1: &str = "0x0d4a11d5eeaac28ec3f61d100daf4d40471f1852";
}

/// Parse hex string to Bytes.
fn hex_to_bytes(hex: &str) -> Bytes {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    let decoded = hex::decode(hex).unwrap_or_default();
    Bytes::from(decoded)
}

/// Create a mock Token for testing.
pub fn create_mock_token(address: &str, symbol: &str, decimals: u8) -> Token {
    Token {
        address: hex_to_bytes(address),
        decimals: decimals as u32,
        symbol: symbol.to_string(),
        gas: vec![Some(50_000)],
        tax: 0,
        chain: Chain::Ethereum.into(),
        quality: 100,
    }
}

/// Create a mock ProtocolComponent for testing.
///
/// Uses `ProtocolComponent::new()` to avoid deprecated field warnings.
pub fn create_mock_component(id: &str, protocol: &str, tokens: Vec<Token>) -> ProtocolComponent {
    let id_bytes = hex_to_bytes(id);
    ProtocolComponent::new(
        id_bytes.clone(),
        protocol.to_string(),
        "pool".to_string(),
        Chain::Ethereum.into(),
        tokens,
        vec![id_bytes],
        Default::default(),
        Bytes::default(),
        chrono::Utc::now().naive_utc(),
    )
}

/// Test fixture for building BridgeService with mock data.
#[allow(dead_code)]
pub struct TestFixture {
    pub state_cache: Arc<StateCache>,
    pub transient_cache: Arc<TransientStateCache>,
}

#[allow(dead_code)]
impl TestFixture {
    pub fn new() -> Self {
        Self {
            state_cache: Arc::new(StateCache::new()),
            transient_cache: Arc::new(TransientStateCache::with_default_ttl()),
        }
    }

    /// Add a pool with tokens and state.
    pub fn add_pool(
        &self,
        chain_id: u64,
        pool_id: &str,
        protocol: &str,
        tokens: Vec<Token>,
        state: MockProtocolSim,
    ) {
        let component = create_mock_component(pool_id, protocol, tokens);
        self.state_cache
            .insert_component_for_chain(chain_id, pool_id.to_string(), component);
        self.state_cache
            .insert_state_for_chain(chain_id, pool_id.to_string(), Box::new(state));
    }

    /// Add a standard WETH/USDC pool.
    pub fn add_weth_usdc_pool(
        &self,
        chain_id: u64,
        pool_id: &str,
        weth_reserve: u64,
        usdc_reserve: u64,
    ) {
        let weth = create_mock_token(addresses::WETH, "WETH", 18);
        let usdc = create_mock_token(addresses::USDC, "USDC", 6);
        let state = MockProtocolSim::new(weth_reserve, usdc_reserve);

        self.add_pool(chain_id, pool_id, "uniswap_v2", vec![weth, usdc], state);
    }

    /// Mark snapshot complete.
    pub fn complete_snapshot(&self, chain_id: u64) {
        self.state_cache
            .mark_snapshot_complete_for_chain(chain_id, std::time::Duration::from_millis(100));
    }

    /// Build a BridgeService with mocks.
    pub fn build_bridge_service(&self, chain_ids: Vec<u64>) -> BridgeService {
        use tycho_core::manager::{ChainConfig, TychoManager};

        let chain_configs: Vec<ChainConfig> = chain_ids
            .iter()
            .filter_map(|&id| {
                let chain = match id {
                    1 => Some(Chain::Ethereum),
                    8453 => Some(Chain::Base),
                    _ => None,
                }?;
                Some(ChainConfig::new(chain))
            })
            .collect();

        let tycho_manager = TychoManager::with_chain_configs(
            "test-url.example.com".to_string(),
            "test-api-key".to_string(),
            chain_configs,
            Arc::clone(&self.state_cache),
        );

        BridgeService::new(
            Arc::clone(&self.state_cache),
            Arc::new(tycho_manager),
            Arc::clone(&self.transient_cache),
            chain_ids,
            "test-url.example.com".to_string(),
        )
    }
}
