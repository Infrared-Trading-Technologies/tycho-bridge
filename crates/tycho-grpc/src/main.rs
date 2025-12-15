//! Tycho Bridge gRPC Server Binary.
//!
//! This is the main entry point for the gRPC server.
//! It initializes logging, loads configuration, connects to Tycho,
//! and starts the gRPC server.

use std::sync::Arc;

use tonic::transport::Server;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use tycho_core::{BridgeService, Config, StateCache, TransientStateCache, TychoManager};
use tycho_grpc::{GrpcService, TychoBridgeServer};

#[tokio::main]
async fn main() {
    // Initialize structured logging (12-factor: logs as event streams)
    init_tracing();

    info!("Starting Tycho Bridge gRPC server");

    // Load configuration from environment (12-factor: config in env vars)
    let config = match Config::from_env() {
        Ok(cfg) => {
            info!(config = ?cfg, "Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!(error = %e, "Failed to load configuration from environment");
            std::process::exit(1);
        }
    };

    // Validate chain IDs early
    let chain_ids = match config.parsed_chain_ids() {
        Ok(ids) => {
            info!(?ids, "Target chains configured");
            ids
        }
        Err(e) => {
            error!(error = %e, "Invalid chain ID configuration");
            std::process::exit(1);
        }
    };

    // Initialize Prometheus metrics exporter
    if let Err(e) = tycho_core::metrics::init_metrics(config.metrics_port) {
        error!(error = %e, "Failed to initialize metrics exporter");
        std::process::exit(1);
    }

    info!(
        grpc_addr = %config.grpc_addr(),
        metrics_port = config.metrics_port,
        "Tycho Bridge initialized"
    );

    // Create shared state cache
    let state_cache = Arc::new(StateCache::new());
    info!("State cache initialized");

    // Initialize TychoManager with shared state cache
    let tycho_manager = match TychoManager::new(&config, Arc::clone(&state_cache)) {
        Ok(manager) => {
            info!(?chain_ids, "TychoManager created successfully");
            manager
        }
        Err(e) => {
            error!(error = %e, "Failed to create TychoManager");
            std::process::exit(1);
        }
    };

    // Connect to Tycho Indexer
    info!("Connecting to Tycho Indexer...");
    let stream_handle = match tycho_manager.connect().await {
        Ok(handle) => {
            info!("Successfully initiated connection to Tycho Indexer");
            handle
        }
        Err(e) => {
            error!(error = %e, "Failed to connect to Tycho Indexer");
            std::process::exit(1);
        }
    };

    // Wait for the stream to be fully connected
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let status = tycho_manager.status().await;
    info!(?status, "Current connection status");

    if let Some(block) = tycho_manager.latest_block().await {
        info!(block, "Latest block received");
    }

    // Log state cache statistics
    let cache_stats = state_cache.stats();
    info!(
        state_count = cache_stats.state_count,
        component_count = cache_stats.component_count,
        latest_block = cache_stats.latest_block,
        update_count = cache_stats.update_count,
        snapshot_complete = cache_stats.snapshot_complete,
        snapshot_duration_ms = ?cache_stats.snapshot_duration_ms,
        "State cache statistics"
    );

    let token_count = tycho_manager.tokens().await.len();
    info!(token_count, "Total tokens loaded");

    // Wrap TychoManager in Arc for shared access
    let tycho_manager = Arc::new(tycho_manager);

    // Create transient state cache for simulation state chaining
    let transient_state_cache = Arc::new(TransientStateCache::with_default_ttl());
    info!(
        ttl_secs = transient_state_cache.ttl().as_secs(),
        "Transient state cache initialized"
    );

    // Spawn background cleanup task for expired transient states
    let _cleanup_handle = tycho_core::state::spawn_cleanup_task(Arc::clone(&transient_state_cache));

    // Create BridgeService with dependency injection
    let bridge_service = BridgeService::new(
        Arc::clone(&state_cache),
        Arc::clone(&tycho_manager),
        Arc::clone(&transient_state_cache),
        chain_ids,
        config.tycho_url.clone(),
    );

    // Create gRPC service wrapping BridgeService
    let grpc_service = GrpcService::new(Arc::new(bridge_service));

    // Parse gRPC server address
    let grpc_addr = config.grpc_addr().parse().unwrap_or_else(|e| {
        error!(error = %e, addr = %config.grpc_addr(), "Invalid gRPC address");
        std::process::exit(1);
    });

    info!(grpc_addr = %config.grpc_addr(), "Starting gRPC server");

    // Start gRPC server (12-factor: port binding)
    // Run the server and stream handler concurrently
    tokio::select! {
        result = Server::builder()
            .add_service(TychoBridgeServer::new(grpc_service))
            .serve(grpc_addr) => {
            match result {
                Ok(()) => info!("gRPC server shut down gracefully"),
                Err(e) => error!(error = %e, "gRPC server error"),
            }
        }
        result = stream_handle => {
            match result {
                Ok(()) => warn!("Stream task completed unexpectedly"),
                Err(e) => error!(error = %e, "Stream task panicked"),
            }
        }
    }

    info!("Tycho Bridge service shutting down");
}

/// Initialize the tracing subscriber with environment-based filtering.
///
/// Follows 12-factor principle: logs as event streams to stdout.
fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}
