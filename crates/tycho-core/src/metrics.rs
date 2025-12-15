//! Prometheus metrics for Tycho Bridge observability.
//!
//! This module provides centralized metric definitions for monitoring the service.
//! Metrics are exposed via a Prometheus-compatible HTTP endpoint.
//!
//! # Metrics Overview
//!
//! | Metric Name                          | Type      | Description                                |
//! |--------------------------------------|-----------|---------------------------------------------|
//! | `tycho_client_connected`             | Gauge     | Connection status (1=connected, 0=disconnected) |
//! | `tycho_state_update_latency_ms`      | Histogram | Latency of processing state updates in ms   |
//! | `tycho_simulation_duration_us`       | Histogram | Duration of swap simulations in microseconds |
//! | `tycho_cache_state_count`            | Gauge     | Number of states in the cache               |
//! | `tycho_cache_component_count`        | Gauge     | Number of components in the cache           |
//! | `tycho_grpc_requests_total`          | Counter   | Total number of gRPC requests by method     |
//! | `tycho_grpc_errors_total`            | Counter   | Total number of gRPC errors by method       |
//!
//! # 12-Factor Compliance
//!
//! - Port binding: Metrics are exposed on a configurable port (default 9000)
//! - Logs as event streams: Metrics complement structured logging
//! - Config: All settings from environment variables
//!
//! # SOLID Compliance
//!
//! - Single Responsibility: Only handles metric definitions and initialization
//! - Open/Closed: New metrics can be added without modifying existing code
//! - Interface Segregation: Uses simple macros for metric recording

use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info};

/// Default port for Prometheus metrics endpoint.
pub const DEFAULT_METRICS_PORT: u16 = 9000;

/// Metric name for Tycho client connection status.
pub const TYCHO_CLIENT_CONNECTED: &str = "tycho_client_connected";

/// Metric name for state update processing latency.
pub const STATE_UPDATE_LATENCY_MS: &str = "tycho_state_update_latency_ms";

/// Metric name for swap simulation duration.
pub const SIMULATION_DURATION_US: &str = "tycho_simulation_duration_us";

/// Metric name for cache state count.
pub const CACHE_STATE_COUNT: &str = "tycho_cache_state_count";

/// Metric name for cache component count.
pub const CACHE_COMPONENT_COUNT: &str = "tycho_cache_component_count";

/// Metric name for total gRPC requests.
pub const GRPC_REQUESTS_TOTAL: &str = "tycho_grpc_requests_total";

/// Metric name for total gRPC errors.
pub const GRPC_ERRORS_TOTAL: &str = "tycho_grpc_errors_total";

/// Metric name for latest block number.
pub const LATEST_BLOCK_NUMBER: &str = "tycho_latest_block_number";

/// Metric name for token count.
pub const TOKEN_COUNT: &str = "tycho_token_count";

/// Metric name for snapshot duration.
pub const SNAPSHOT_DURATION_MS: &str = "tycho_snapshot_duration_ms";

/// Histogram buckets for simulation duration (microseconds).
/// Ranges from 1µs to 100ms to capture fast and slow simulations.
const SIMULATION_BUCKETS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 25000.0,
    50000.0, 100000.0,
];

/// Histogram buckets for state update latency (milliseconds).
/// Ranges from 1ms to 10s to capture typical blockchain update patterns.
const UPDATE_LATENCY_BUCKETS: &[f64] = &[
    1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
];

/// Initialize the Prometheus metrics exporter.
///
/// This function sets up the metrics recording infrastructure and starts
/// an HTTP listener on the specified port for Prometheus scraping.
///
/// # Arguments
///
/// * `metrics_port` - The port to expose the /metrics endpoint on
///
/// # Errors
///
/// Returns an error if the exporter fails to install (e.g., port already in use).
///
/// # Example
///
/// ```ignore
/// init_metrics(9000)?;
/// // Now metrics can be recorded and scraped at http://0.0.0.0:9000/metrics
/// ```
pub fn init_metrics(metrics_port: u16) -> Result<(), MetricsError> {
    let addr: SocketAddr = ([0, 0, 0, 0], metrics_port).into();

    info!(
        port = metrics_port,
        "Initializing Prometheus metrics exporter"
    );

    PrometheusBuilder::new()
        .with_http_listener(addr)
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix(SIMULATION_DURATION_US.to_string()),
            SIMULATION_BUCKETS,
        )
        .map_err(|e| MetricsError::Configuration(e.to_string()))?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix(STATE_UPDATE_LATENCY_MS.to_string()),
            UPDATE_LATENCY_BUCKETS,
        )
        .map_err(|e| MetricsError::Configuration(e.to_string()))?
        .install()
        .map_err(|e| {
            error!(error = %e, "Failed to install Prometheus exporter");
            MetricsError::Installation(e.to_string())
        })?;

    info!(
        addr = %addr,
        "Prometheus metrics endpoint started"
    );

    Ok(())
}

/// Errors that can occur during metrics initialization.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// Failed to configure the metrics exporter.
    #[error("metrics configuration error: {0}")]
    Configuration(String),

    /// Failed to install the metrics exporter.
    #[error("metrics installation error: {0}")]
    Installation(String),
}

// ============================================================================
// Metric Recording Helper Functions
// ============================================================================
// These functions provide a clean API for recording metrics throughout the codebase.
// Using functions rather than direct macro calls improves maintainability and
// allows for future changes to metric implementation without modifying call sites.

/// Record the Tycho client connection status.
///
/// # Arguments
///
/// * `chain_id` - The chain ID for this connection
/// * `connected` - Whether the client is connected (true) or disconnected (false)
#[inline]
pub fn record_connection_status(chain_id: u64, connected: bool) {
    let value = if connected { 1.0 } else { 0.0 };
    gauge!(TYCHO_CLIENT_CONNECTED, "chain_id" => chain_id.to_string()).set(value);
}

/// Record the latency of processing a state update.
///
/// # Arguments
///
/// * `chain_id` - The chain ID for this update
/// * `duration` - The time taken to process the update
#[inline]
pub fn record_state_update_latency(chain_id: u64, duration: Duration) {
    histogram!(
        STATE_UPDATE_LATENCY_MS,
        "chain_id" => chain_id.to_string()
    )
    .record(duration.as_secs_f64() * 1000.0);
}

/// Record the duration of a swap simulation.
///
/// # Arguments
///
/// * `chain_id` - The chain ID where the simulation was performed
/// * `pool_id` - The pool ID (first 8 chars for cardinality control)
/// * `duration` - The simulation duration
#[inline]
pub fn record_simulation_duration(chain_id: u64, pool_id: &str, duration: Duration) {
    // Truncate pool_id to avoid high cardinality
    let pool_prefix = if pool_id.len() > 10 {
        &pool_id[..10]
    } else {
        pool_id
    };

    histogram!(
        SIMULATION_DURATION_US,
        "chain_id" => chain_id.to_string(),
        "pool_prefix" => pool_prefix.to_string()
    )
    .record(duration.as_secs_f64() * 1_000_000.0);
}

/// Update the cache state count gauge.
///
/// # Arguments
///
/// * `chain_id` - Optional chain ID for per-chain tracking
/// * `count` - The current number of states in the cache
#[inline]
pub fn set_cache_state_count(chain_id: Option<u64>, count: usize) {
    match chain_id {
        Some(id) => {
            gauge!(CACHE_STATE_COUNT, "chain_id" => id.to_string()).set(count as f64);
        }
        None => {
            gauge!(CACHE_STATE_COUNT, "chain_id" => "all").set(count as f64);
        }
    }
}

/// Update the cache component count gauge.
///
/// # Arguments
///
/// * `chain_id` - Optional chain ID for per-chain tracking
/// * `count` - The current number of components in the cache
#[inline]
pub fn set_cache_component_count(chain_id: Option<u64>, count: usize) {
    match chain_id {
        Some(id) => {
            gauge!(CACHE_COMPONENT_COUNT, "chain_id" => id.to_string()).set(count as f64);
        }
        None => {
            gauge!(CACHE_COMPONENT_COUNT, "chain_id" => "all").set(count as f64);
        }
    }
}

/// Increment the gRPC request counter.
///
/// # Arguments
///
/// * `method` - The gRPC method name (e.g., "GetPools", "SimulateSwap")
#[inline]
pub fn inc_grpc_requests(method: &str) {
    counter!(GRPC_REQUESTS_TOTAL, "method" => method.to_string()).increment(1);
}

/// Increment the gRPC error counter.
///
/// # Arguments
///
/// * `method` - The gRPC method name
/// * `code` - The error code or type
#[inline]
pub fn inc_grpc_errors(method: &str, code: &str) {
    counter!(
        GRPC_ERRORS_TOTAL,
        "method" => method.to_string(),
        "code" => code.to_string()
    )
    .increment(1);
}

/// Update the latest block number gauge.
///
/// # Arguments
///
/// * `chain_id` - The chain ID
/// * `block_number` - The latest block number
#[inline]
pub fn set_latest_block_number(chain_id: u64, block_number: u64) {
    gauge!(LATEST_BLOCK_NUMBER, "chain_id" => chain_id.to_string()).set(block_number as f64);
}

/// Update the token count gauge.
///
/// # Arguments
///
/// * `chain_id` - The chain ID
/// * `count` - The number of tokens loaded
#[inline]
pub fn set_token_count(chain_id: u64, count: usize) {
    gauge!(TOKEN_COUNT, "chain_id" => chain_id.to_string()).set(count as f64);
}

/// Record the snapshot ingestion duration.
///
/// # Arguments
///
/// * `chain_id` - The chain ID
/// * `duration_ms` - The duration in milliseconds
#[inline]
pub fn record_snapshot_duration(chain_id: u64, duration_ms: u64) {
    gauge!(SNAPSHOT_DURATION_MS, "chain_id" => chain_id.to_string()).set(duration_ms as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_names_are_valid() {
        // Prometheus metric names must match [a-zA-Z_:][a-zA-Z0-9_:]*
        let names = [
            TYCHO_CLIENT_CONNECTED,
            STATE_UPDATE_LATENCY_MS,
            SIMULATION_DURATION_US,
            CACHE_STATE_COUNT,
            CACHE_COMPONENT_COUNT,
            GRPC_REQUESTS_TOTAL,
            GRPC_ERRORS_TOTAL,
            LATEST_BLOCK_NUMBER,
            TOKEN_COUNT,
            SNAPSHOT_DURATION_MS,
        ];

        for name in names {
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':'),
                "Invalid metric name: {}",
                name
            );
            assert!(
                !name.chars().next().unwrap().is_ascii_digit(),
                "Metric name cannot start with digit: {}",
                name
            );
        }
    }

    #[test]
    fn test_simulation_buckets_are_sorted() {
        for i in 1..SIMULATION_BUCKETS.len() {
            assert!(
                SIMULATION_BUCKETS[i] > SIMULATION_BUCKETS[i - 1],
                "Buckets must be in ascending order"
            );
        }
    }

    #[test]
    fn test_update_latency_buckets_are_sorted() {
        for i in 1..UPDATE_LATENCY_BUCKETS.len() {
            assert!(
                UPDATE_LATENCY_BUCKETS[i] > UPDATE_LATENCY_BUCKETS[i - 1],
                "Buckets must be in ascending order"
            );
        }
    }

    #[test]
    fn test_default_metrics_port() {
        assert_eq!(DEFAULT_METRICS_PORT, 9000);
    }
}
