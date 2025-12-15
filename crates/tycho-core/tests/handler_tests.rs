//! Integration tests for simulation handlers.
//!
//! These tests verify the BridgeService handlers work correctly with mock data,
//! without requiring network connectivity.

mod common;

use num_bigint::BigUint;
use tycho_core::error::BridgeError;
use tycho_core::types::request::{
    BatchGetAmountOutRequest, GetAmountOutRequest, GetFeeRequest, GetLimitsRequest,
    SimulateRouteRequest, SpotPriceRequest,
};

use common::{addresses, TestFixture};

// =============================================================================
// SpotPrice Handler Tests
// =============================================================================

#[test]
fn test_spot_price_success() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = SpotPriceRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        base_token: addresses::WETH.to_string(),
        quote_token: addresses::USDC.to_string(),
    };

    let result = service.spot_price(req);
    assert!(result.is_ok());

    let response = result.unwrap();
    // Price should be reserve1/reserve0 = 2.0
    let price = response.numerator.clone().to_string().parse::<f64>().unwrap()
        / response.denominator.clone().to_string().parse::<f64>().unwrap();
    assert!((price - 2.0).abs() < 0.1, "Expected price ~2.0, got {}", price);
}

#[test]
fn test_spot_price_component_not_found() {
    let fixture = TestFixture::new();
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = SpotPriceRequest {
        chain_id: 1,
        component_id: "nonexistent-pool".to_string(),
        base_token: addresses::WETH.to_string(),
        quote_token: addresses::USDC.to_string(),
    };

    let result = service.spot_price(req);
    assert!(result.is_err());

    match result.unwrap_err() {
        BridgeError::ComponentNotFound { component_id } => {
            assert_eq!(component_id, "nonexistent-pool");
        }
        other => panic!("Expected ComponentNotFound, got {:?}", other),
    }
}

#[test]
fn test_spot_price_chain_not_supported() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    // Only configure chain 1, but request for chain 42
    let service = fixture.build_bridge_service(vec![1]);

    let req = SpotPriceRequest {
        chain_id: 42,
        component_id: addresses::POOL_1.to_string(),
        base_token: addresses::WETH.to_string(),
        quote_token: addresses::USDC.to_string(),
    };

    let result = service.spot_price(req);
    assert!(result.is_err());

    match result.unwrap_err() {
        BridgeError::ChainNotSupported { chain_id } => {
            assert_eq!(chain_id, 42);
        }
        other => panic!("Expected ChainNotSupported, got {:?}", other),
    }
}

// =============================================================================
// GetAmountOut Handler Tests
// =============================================================================

#[test]
fn test_get_amount_out_success() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = GetAmountOutRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: addresses::WETH.to_string(),
        token_out: addresses::USDC.to_string(),
        amount_in: BigUint::from(100_000u64),
        save_state: false,
        state_id: None,
    };

    let result = service.get_amount_out(req);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.amount_out > BigUint::from(0u64));
    assert!(response.gas_cost > BigUint::from(0u64));
    assert!(response.state_id.is_none()); // save_state was false
}

#[test]
fn test_get_amount_out_with_state_save() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = GetAmountOutRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: addresses::WETH.to_string(),
        token_out: addresses::USDC.to_string(),
        amount_in: BigUint::from(100_000u64),
        save_state: true,
        state_id: None,
    };

    let result = service.get_amount_out(req);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.state_id.is_some(), "State should be saved");

    // Verify the state was stored in transient cache
    let state_id = response.state_id.unwrap();
    let cached_state = fixture.transient_cache.get(&state_id);
    assert!(cached_state.is_some(), "State should be in transient cache");
}

#[test]
fn test_get_amount_out_state_chaining() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    // First swap - save state
    let req1 = GetAmountOutRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: addresses::WETH.to_string(),
        token_out: addresses::USDC.to_string(),
        amount_in: BigUint::from(100_000u64),
        save_state: true,
        state_id: None,
    };

    let result1 = service.get_amount_out(req1).unwrap();
    let first_amount_out = result1.amount_out.clone();
    let state_id = result1.state_id.expect("Should have state_id");

    // Second swap - use saved state (pool has less reserves now)
    let req2 = GetAmountOutRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: addresses::WETH.to_string(),
        token_out: addresses::USDC.to_string(),
        amount_in: BigUint::from(100_000u64),
        save_state: false,
        state_id: Some(state_id),
    };

    let result2 = service.get_amount_out(req2).unwrap();
    let second_amount_out = result2.amount_out;

    // Due to constant product formula, second swap should get less out
    // (pool is "used" after first swap)
    assert!(
        second_amount_out < first_amount_out,
        "Second swap should get less due to price impact. First: {}, Second: {}",
        first_amount_out,
        second_amount_out
    );
}

#[test]
fn test_get_amount_out_zero_amount() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = GetAmountOutRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: addresses::WETH.to_string(),
        token_out: addresses::USDC.to_string(),
        amount_in: BigUint::from(0u64),
        save_state: false,
        state_id: None,
    };

    let result = service.get_amount_out(req);
    assert!(result.is_err());

    match result.unwrap_err() {
        BridgeError::InvalidArgument { message } => {
            assert!(message.contains("amount_in"));
        }
        other => panic!("Expected InvalidArgument, got {:?}", other),
    }
}

#[test]
fn test_get_amount_out_token_not_found() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = GetAmountOutRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: "0x0000000000000000000000000000000000000001".to_string(), // Unknown token
        token_out: addresses::USDC.to_string(),
        amount_in: BigUint::from(100_000u64),
        save_state: false,
        state_id: None,
    };

    let result = service.get_amount_out(req);
    assert!(result.is_err());

    match result.unwrap_err() {
        BridgeError::TokenNotFound { .. } => {}
        other => panic!("Expected TokenNotFound, got {:?}", other),
    }
}

// =============================================================================
// GetFee Handler Tests
// =============================================================================

#[test]
fn test_get_fee_success() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = GetFeeRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token: addresses::WETH.to_string(),
    };

    let result = service.get_fee(req);
    assert!(result.is_ok());

    let response = result.unwrap();
    // MockProtocolSim has 0.3% fee = 30 bps
    let fee_bps = response.fee_numerator.clone() * BigUint::from(10000u32)
        / response.fee_denominator.clone();
    assert_eq!(fee_bps, BigUint::from(30u32), "Expected 30 bps fee");
}

// =============================================================================
// GetLimits Handler Tests
// =============================================================================

#[test]
fn test_get_limits_success() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = GetLimitsRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        token_in: addresses::WETH.to_string(),
        token_out: addresses::USDC.to_string(),
    };

    let result = service.get_limits(req);
    assert!(result.is_ok());

    let response = result.unwrap();
    // MockProtocolSim returns reserves as limits
    assert_eq!(response.max_amount_in, BigUint::from(1_000_000u64));
}

// =============================================================================
// BatchGetAmountOut Handler Tests
// =============================================================================

#[test]
fn test_batch_get_amount_out_success() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let requests = vec![
        GetAmountOutRequest {
            chain_id: 1,
            component_id: addresses::POOL_1.to_string(),
            token_in: addresses::WETH.to_string(),
            token_out: addresses::USDC.to_string(),
            amount_in: BigUint::from(100_000u64),
            save_state: false,
            state_id: None,
        },
        GetAmountOutRequest {
            chain_id: 1,
            component_id: addresses::POOL_1.to_string(),
            token_in: addresses::WETH.to_string(),
            token_out: addresses::USDC.to_string(),
            amount_in: BigUint::from(200_000u64),
            save_state: false,
            state_id: None,
        },
    ];

    let batch_req = BatchGetAmountOutRequest { requests };

    let result = service.batch_get_amount_out(batch_req);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.results.len(), 2);
    assert!(response.results[0].is_some());
    assert!(response.results[1].is_some());

    // Second request with more input should get more output
    let first_out = response.results[0].as_ref().unwrap().amount_out.clone();
    let second_out = response.results[1].as_ref().unwrap().amount_out.clone();
    assert!(second_out > first_out);
}

#[test]
fn test_batch_get_amount_out_partial_failure() {
    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let requests = vec![
        // Valid request
        GetAmountOutRequest {
            chain_id: 1,
            component_id: addresses::POOL_1.to_string(),
            token_in: addresses::WETH.to_string(),
            token_out: addresses::USDC.to_string(),
            amount_in: BigUint::from(100_000u64),
            save_state: false,
            state_id: None,
        },
        // Invalid request - nonexistent pool
        GetAmountOutRequest {
            chain_id: 1,
            component_id: "nonexistent-pool".to_string(),
            token_in: addresses::WETH.to_string(),
            token_out: addresses::USDC.to_string(),
            amount_in: BigUint::from(100_000u64),
            save_state: false,
            state_id: None,
        },
    ];

    let batch_req = BatchGetAmountOutRequest { requests };

    let result = service.batch_get_amount_out(batch_req);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.results.len(), 2);
    assert!(response.results[0].is_some()); // First succeeded
    assert!(response.results[1].is_none()); // Second failed
}

// =============================================================================
// SimulateRoute Handler Tests
// =============================================================================

#[test]
fn test_simulate_route_single_hop() {
    use tycho_core::types::request::RouteHop;

    let fixture = TestFixture::new();
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = SimulateRouteRequest {
        chain_id: 1,
        amount_in: BigUint::from(100_000u64),
        hops: vec![RouteHop {
            component_id: addresses::POOL_1.to_string(),
            token_in: addresses::WETH.to_string(),
            token_out: addresses::USDC.to_string(),
        }],
        state_overrides: Default::default(),
    };

    let result = service.simulate_route(req);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.hop_results.len(), 1);
    assert!(response.final_amount_out > BigUint::from(0u64));
    assert!(response.total_gas_cost > BigUint::from(0u64));
}

#[test]
fn test_simulate_route_multi_hop() {
    use tycho_core::types::request::RouteHop;

    let fixture = TestFixture::new();
    // Create two pools for multi-hop
    let pool_2 = "0x0000000000000000000000000000000000000002";
    let dai_addr = "0x6B175474E89094C44Da98b954EedeCAcB5dC4430"; // Real DAI

    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000);
    // Second pool: USDC -> DAI
    fixture.add_pool(
        1,
        pool_2,
        "uniswap_v2",
        vec![
            common::create_mock_token(addresses::USDC, "USDC", 6),
            common::create_mock_token(dai_addr, "DAI", 18),
        ],
        common::MockProtocolSim::new(500_000, 500_000),
    );
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = SimulateRouteRequest {
        chain_id: 1,
        amount_in: BigUint::from(100_000u64),
        hops: vec![
            RouteHop {
                component_id: addresses::POOL_1.to_string(),
                token_in: addresses::WETH.to_string(),
                token_out: addresses::USDC.to_string(),
            },
            RouteHop {
                component_id: pool_2.to_string(),
                token_in: addresses::USDC.to_string(),
                token_out: dai_addr.to_string(),
            },
        ],
        state_overrides: Default::default(),
    };

    let result = service.simulate_route(req);
    assert!(result.is_ok(), "Multi-hop simulation failed: {:?}", result.err());

    let response = result.unwrap();
    assert_eq!(response.hop_results.len(), 2);
    assert!(response.final_amount_out > BigUint::from(0u64));
    // Total gas should be sum of both hops
    assert!(response.total_gas_cost >= BigUint::from(200_000u64));
}

#[test]
fn test_simulate_route_empty_hops() {
    let fixture = TestFixture::new();
    fixture.complete_snapshot(1);

    let service = fixture.build_bridge_service(vec![1]);

    let req = SimulateRouteRequest {
        chain_id: 1,
        amount_in: BigUint::from(100_000u64),
        hops: vec![],
        state_overrides: Default::default(),
    };

    let result = service.simulate_route(req);
    assert!(result.is_err());

    match result.unwrap_err() {
        BridgeError::InvalidArgument { message } => {
            assert!(message.contains("hop"));
        }
        other => panic!("Expected InvalidArgument, got {:?}", other),
    }
}

// =============================================================================
// Multi-Chain Tests
// =============================================================================

#[test]
fn test_multi_chain_isolation() {
    let fixture = TestFixture::new();

    // Add same pool ID on different chains with different reserves
    fixture.add_weth_usdc_pool(1, addresses::POOL_1, 1_000_000, 2_000_000); // Ethereum
    fixture.add_weth_usdc_pool(8453, addresses::POOL_1, 500_000, 1_500_000); // Base
    fixture.complete_snapshot(1);
    fixture.complete_snapshot(8453);

    let service = fixture.build_bridge_service(vec![1, 8453]);

    // Query Ethereum pool
    let req_eth = SpotPriceRequest {
        chain_id: 1,
        component_id: addresses::POOL_1.to_string(),
        base_token: addresses::WETH.to_string(),
        quote_token: addresses::USDC.to_string(),
    };

    let result_eth = service.spot_price(req_eth).unwrap();

    // Query Base pool (same pool ID, different chain)
    let req_base = SpotPriceRequest {
        chain_id: 8453,
        component_id: addresses::POOL_1.to_string(),
        base_token: addresses::WETH.to_string(),
        quote_token: addresses::USDC.to_string(),
    };

    let result_base = service.spot_price(req_base).unwrap();

    // Prices should be different due to different reserves
    // Ethereum: 2_000_000 / 1_000_000 = 2.0
    // Base: 1_500_000 / 500_000 = 3.0
    let price_eth = result_eth.numerator.clone().to_string().parse::<f64>().unwrap()
        / result_eth.denominator.clone().to_string().parse::<f64>().unwrap();
    let price_base = result_base.numerator.clone().to_string().parse::<f64>().unwrap()
        / result_base.denominator.clone().to_string().parse::<f64>().unwrap();

    assert!(
        (price_eth - 2.0).abs() < 0.1,
        "Ethereum price should be ~2.0"
    );
    assert!((price_base - 3.0).abs() < 0.1, "Base price should be ~3.0");
}
