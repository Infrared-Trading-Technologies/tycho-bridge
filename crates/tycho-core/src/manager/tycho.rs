//! Tycho Manager - WebSocket lifecycle and state management.
//!
//! This module is responsible for:
//! 1. Connecting to the hosted Tycho Indexer via WebSocket
//! 2. Authenticating with the provided API key
//! 3. Managing the stream lifecycle (connect, reconnect, shutdown)
//! 4. Loading initial token data via RPC
//! 5. Populating and maintaining the StateCache with ProtocolSim states
//!
//! # Architecture
//!
//! The `TychoManager` follows SOLID principles:
//! - **Single Responsibility**: Manages only the Tycho connection and stream lifecycle
//! - **Open/Closed**: New protocols can be added via configuration, not code changes
//! - **Dependency Inversion**: Depends on abstractions (traits) not concrete implementations
//!
//! # State Flow
//!
//! ```text
//! [Tycho Indexer] --WebSocket--> [TychoManager] --populate--> [StateCache]
//!                                                                  |
//!                  First message = Full snapshot (all pools)       |
//!                  Subsequent messages = Delta updates only        v
//!                                              [DashMap<ComponentId, Arc<ProtocolSim>>]
//! ```
//!
//! # Example
//!
//! ```ignore
//! use tycho_bridge::{Config, TychoManager, StateCache};
//! use std::sync::Arc;
//!
//! let config = Config::from_env()?;
//! let state_cache = Arc::new(StateCache::new());
//! let manager = TychoManager::new(&config, Arc::clone(&state_cache))?;
//! let handle = manager.connect().await?;
//!
//! // After connection, query the cache for simulations
//! if let Some(state) = state_cache.get_state("0x...pool") {
//!     let result = state.get_amount_out(amount, &token_in, &token_out)?;
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::domain::{
    Chain, ComponentFilter, HttpRPCClient, HttpRPCClientOptions, ProtocolStreamBuilder, Token,
    UniswapV2State, UniswapV3State,
};
use crate::metrics::{
    record_connection_status, record_snapshot_duration, record_state_update_latency,
    set_cache_state_count, set_latest_block_number, set_token_count,
};
use crate::state::StateCache;

// Import RPCClient trait to access get_all_tokens method
use tycho_simulation::tycho_client::rpc::RPCClient;

// Re-export types needed for external use
pub use tycho_simulation::protocol::models::Update;
pub use tycho_simulation::tycho_common::Bytes;

/// Default TVL filter range in ETH for component filtering.
/// Components below this threshold are ignored to reduce noise.
const DEFAULT_TVL_THRESHOLD: f64 = 10.0;

/// Minimum token quality score (0-100) for filtering tokens.
/// 100 = Normal, 75 = Rebase, 50 = Fee, 10 = Analysis failed at creation.
const MIN_TOKEN_QUALITY: i32 = 51;

/// Number of days since last trade to filter tokens.
const TOKEN_LAST_TRADED_DAYS: u64 = 30;

/// Pagination chunk size for token fetching.
const TOKEN_FETCH_CHUNK_SIZE: usize = 1000;

/// Startup timeout for the stream in seconds.
const STREAM_STARTUP_TIMEOUT_SECS: u64 = 120;

/// Errors that can occur during Tycho Manager operations.
#[derive(Debug, Error)]
pub enum TychoManagerError {
    /// Failed to parse chain ID to Chain enum.
    #[error("unsupported chain ID: {0}")]
    UnsupportedChain(u64),

    /// Failed to connect to Tycho indexer.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Failed to load tokens from RPC.
    #[error("token loading failed: {0}")]
    TokenLoadingFailed(String),

    /// Stream build error.
    #[error("stream build failed: {0}")]
    StreamBuildFailed(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigError(String),
}

/// Token registry keyed by token address.
pub type TokenRegistry = HashMap<Bytes, Token>;

/// Chain-partitioned token registry.
/// Outer key is chain ID, inner value is a TokenRegistry.
pub type ChainTokenRegistry = HashMap<u64, TokenRegistry>;

/// Connection status for monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Not yet connected.
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Successfully connected and receiving updates.
    Connected,
    /// Connection lost, attempting to reconnect.
    Reconnecting,
    /// Gracefully shut down.
    Shutdown,
}

/// Internal state shared between the manager and background tasks.
#[derive(Debug)]
struct ManagerState {
    /// Current connection status.
    status: ConnectionStatus,
    /// Loaded tokens keyed by chain ID, then by token address.
    tokens: ChainTokenRegistry,
    /// Latest block number received (also tracked in StateCache).
    latest_block: Option<u64>,
}

impl Default for ManagerState {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            tokens: HashMap::new(),
            latest_block: None,
        }
    }
}

/// Configuration for a single chain connection.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// The chain to connect to.
    pub chain: Chain,
    /// TVL threshold for filtering components.
    pub tvl_threshold: f64,
    /// List of protocols to subscribe to.
    pub protocols: Vec<String>,
}

impl ChainConfig {
    /// Create a new chain configuration with default protocols.
    pub fn new(chain: Chain) -> Self {
        let protocols = Self::default_protocols_for_chain(&chain);
        Self {
            chain,
            tvl_threshold: DEFAULT_TVL_THRESHOLD,
            protocols,
        }
    }

    /// Get default protocols for a given chain.
    fn default_protocols_for_chain(chain: &Chain) -> Vec<String> {
        match chain {
            Chain::Ethereum => vec![
                "uniswap_v2".to_string(),
                "uniswap_v3".to_string(),
                "sushiswap_v2".to_string(),
            ],
            Chain::Base => vec!["uniswap_v2".to_string(), "uniswap_v3".to_string()],
            Chain::Unichain => vec!["uniswap_v2".to_string(), "uniswap_v3".to_string()],
            _ => vec!["uniswap_v2".to_string()],
        }
    }
}

/// Tycho Manager handles the WebSocket lifecycle and state management.
///
/// This struct is the main entry point for connecting to the Tycho Indexer.
/// It manages:
/// - WebSocket connection lifecycle
/// - Token loading via RPC
/// - Protocol stream subscription
/// - State synchronization to the shared StateCache
///
/// # Thread Safety
///
/// The manager is designed to be shared across threads via `Arc<TychoManager>`.
/// Internal state is protected by `RwLock` for safe concurrent access.
/// The StateCache uses DashMap for lock-free concurrent reads.
pub struct TychoManager {
    /// Tycho indexer URL (without protocol prefix).
    tycho_url: String,
    /// API key for authentication.
    api_key: String,
    /// Chain configurations to connect to.
    chain_configs: Vec<ChainConfig>,
    /// Shared internal state (connection status, tokens).
    state: Arc<RwLock<ManagerState>>,
    /// Shared state cache for protocol simulations.
    /// This is the "source of truth" for all ProtocolSim states.
    state_cache: Arc<StateCache>,
}

impl TychoManager {
    /// Create a new TychoManager from application configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration containing Tycho URL, API key, and chain IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if chain IDs cannot be parsed or are unsupported.
    pub fn new(config: &Config, state_cache: Arc<StateCache>) -> Result<Self, TychoManagerError> {
        let chain_ids = config
            .parsed_chain_ids()
            .map_err(|e| TychoManagerError::ConfigError(e.to_string()))?;

        let chain_configs = chain_ids
            .into_iter()
            .map(|id| {
                let chain = chain_from_id(id)?;
                Ok(ChainConfig::new(chain))
            })
            .collect::<Result<Vec<_>, TychoManagerError>>()?;

        // Extract host from URL (remove wss:// or https:// prefix if present)
        let tycho_url = config
            .tycho_url
            .trim_start_matches("wss://")
            .trim_start_matches("https://")
            .trim_start_matches("ws://")
            .trim_start_matches("http://")
            .to_string();

        Ok(Self {
            tycho_url,
            api_key: config.tycho_api_key.clone(),
            chain_configs,
            state: Arc::new(RwLock::new(ManagerState::default())),
            state_cache,
        })
    }

    /// Create a TychoManager with custom chain configurations.
    ///
    /// This allows for more fine-grained control over which protocols to subscribe to.
    pub fn with_chain_configs(
        tycho_url: String,
        api_key: String,
        chain_configs: Vec<ChainConfig>,
        state_cache: Arc<StateCache>,
    ) -> Self {
        Self {
            tycho_url,
            api_key,
            chain_configs,
            state: Arc::new(RwLock::new(ManagerState::default())),
            state_cache,
        }
    }

    /// Get the current connection status.
    pub async fn status(&self) -> ConnectionStatus {
        self.state.read().await.status
    }

    /// Get the latest block number received.
    pub async fn latest_block(&self) -> Option<u64> {
        self.state.read().await.latest_block
    }

    /// Get a copy of the loaded tokens for all chains.
    pub async fn tokens(&self) -> ChainTokenRegistry {
        self.state.read().await.tokens.clone()
    }

    /// Get tokens for a specific chain.
    pub async fn tokens_for_chain(&self, chain_id: u64) -> TokenRegistry {
        self.state
            .read()
            .await
            .tokens
            .get(&chain_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get total token count across all chains.
    pub async fn token_count(&self) -> usize {
        self.state
            .read()
            .await
            .tokens
            .values()
            .map(|r| r.len())
            .sum()
    }

    /// Get the number of loaded components from the StateCache.
    pub fn component_count(&self) -> usize {
        self.state_cache.component_count()
    }

    /// Get the number of simulation states in the StateCache.
    pub fn state_count(&self) -> usize {
        self.state_cache.state_count()
    }

    /// Get a reference to the shared StateCache.
    ///
    /// Use this to query simulation states from gRPC handlers.
    pub fn state_cache(&self) -> &Arc<StateCache> {
        &self.state_cache
    }

    /// Check if the initial snapshot has been ingested.
    pub fn is_snapshot_complete(&self) -> bool {
        self.state_cache.is_snapshot_complete()
    }

    /// Get snapshot ingestion metrics.
    pub fn snapshot_duration_ms(&self) -> Option<u64> {
        self.state_cache
            .snapshot_duration()
            .map(|d| d.as_millis() as u64)
    }

    /// Load tokens from Tycho RPC for a specific chain.
    ///
    /// This fetches all tokens that meet quality thresholds from the Tycho indexer.
    async fn load_tokens_for_chain(
        &self,
        chain: Chain,
    ) -> Result<TokenRegistry, TychoManagerError> {
        info!(?chain, "Loading tokens from Tycho RPC");

        let rpc_url = format!("https://{}", self.tycho_url);
        let options = HttpRPCClientOptions::new().with_auth_key(Some(self.api_key.clone()));
        let client = HttpRPCClient::new(&rpc_url, options)
            .map_err(|e| TychoManagerError::TokenLoadingFailed(e.to_string()))?;

        let tokens = client
            .get_all_tokens(
                chain,
                Some(MIN_TOKEN_QUALITY),
                Some(TOKEN_LAST_TRADED_DAYS),
                Some(TOKEN_FETCH_CHUNK_SIZE),
                4, // concurrency
            )
            .await
            .map_err(|e| TychoManagerError::TokenLoadingFailed(e.to_string()))?;

        let token_registry: TokenRegistry = tokens
            .into_iter()
            .filter_map(|t| {
                let addr = t.address.clone();
                t.try_into().ok().map(|token| (addr, token))
            })
            .collect();

        info!(
            ?chain,
            token_count = token_registry.len(),
            "Tokens loaded successfully"
        );

        Ok(token_registry)
    }

    /// Connect to the Tycho Indexer and start streaming state updates for all configured chains.
    ///
    /// This method:
    /// 1. Updates status to Connecting
    /// 2. Loads tokens via RPC for each configured chain (in parallel)
    /// 3. Builds and starts a protocol stream for each chain
    /// 4. Spawns background tasks to process updates from each chain
    ///
    /// # Multi-Chain Support
    ///
    /// Each chain gets its own stream and background task. States are stored
    /// in the shared `StateCache` using composite keys (`{chain_id}:{component_id}`)
    /// to prevent collisions between chains.
    ///
    /// # Returns
    ///
    /// Returns a `JoinHandle` that completes when ALL chain streams have ended.
    ///
    /// # Errors
    ///
    /// Returns an error if no chains are configured or if initial token loading fails.
    pub async fn connect(&self) -> Result<JoinHandle<()>, TychoManagerError> {
        // Update status
        {
            let mut state = self.state.write().await;
            state.status = ConnectionStatus::Connecting;
        }

        info!(
            url = %self.tycho_url,
            chains = ?self.chain_configs.iter().map(|c| &c.chain).collect::<Vec<_>>(),
            chain_count = self.chain_configs.len(),
            "Connecting to Tycho Indexer for all configured chains"
        );

        if self.chain_configs.is_empty() {
            return Err(TychoManagerError::ConfigError(
                "No chains configured".to_string(),
            ));
        }

        // Load tokens for all chains in parallel
        let mut token_futures = Vec::new();
        for chain_config in &self.chain_configs {
            let chain = chain_config.chain;
            token_futures.push(async move {
                let result = self.load_tokens_for_chain(chain).await;
                (chain, result)
            });
        }

        let token_results = futures::future::join_all(token_futures).await;

        // Store tokens and check for errors
        for (chain, result) in token_results {
            match result {
                Ok(tokens) => {
                    let chain_id = chain_to_id(&chain);
                    let token_count = tokens.len();
                    {
                        let mut state = self.state.write().await;
                        state.tokens.insert(chain_id, tokens);
                    }
                    // Record token count metric
                    set_token_count(chain_id, token_count);
                    info!(?chain, chain_id, token_count, "Tokens loaded for chain");
                }
                Err(e) => {
                    error!(?chain, error = %e, "Failed to load tokens for chain");
                    return Err(e);
                }
            }
        }

        // Build and spawn a stream for each chain
        let mut chain_handles = Vec::new();

        for chain_config in &self.chain_configs {
            let chain = chain_config.chain;
            let chain_id = chain_to_id(&chain);

            // Get tokens for this chain
            let tokens = {
                let state = self.state.read().await;
                state.tokens.get(&chain_id).cloned().unwrap_or_default()
            };

            // Build the protocol stream for this chain
            let tvl_filter = ComponentFilter::with_tvl_range(
                chain_config.tvl_threshold,
                chain_config.tvl_threshold,
            );

            let mut stream_builder =
                ProtocolStreamBuilder::new(&self.tycho_url, chain_config.chain.into());

            // Add protocols based on configuration
            for protocol in &chain_config.protocols {
                stream_builder =
                    add_protocol_to_stream(stream_builder, protocol, tvl_filter.clone())?;
            }

            // Configure and build the stream
            let protocol_stream = stream_builder
                .auth_key(Some(self.api_key.clone()))
                .skip_state_decode_failures(true)
                .startup_timeout(Duration::from_secs(STREAM_STARTUP_TIMEOUT_SECS))
                .set_tokens(tokens)
                .await
                .build()
                .await
                .map_err(|e| {
                    TychoManagerError::StreamBuildFailed(format!("Chain {:?}: {}", chain, e))
                })?;

            // Spawn background task to process updates for this chain
            let state = Arc::clone(&self.state);
            let state_cache = Arc::clone(&self.state_cache);
            let handle = tokio::spawn(async move {
                info!(?chain, chain_id, "Protocol stream started for chain");

                // Pin the stream for iteration
                let mut stream = Box::pin(protocol_stream);

                // Track timing for initial snapshot
                let snapshot_start = Instant::now();
                let mut first_message = true;

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(update) => {
                            // Process the update into the StateCache with chain context
                            process_update_for_chain(
                                &state,
                                &state_cache,
                                chain_id,
                                update,
                                first_message,
                                snapshot_start,
                            )
                            .await;

                            if first_message {
                                first_message = false;
                                info!(?chain, chain_id, "First message received for chain");
                            }
                        }
                        Err(e) => {
                            error!(?chain, chain_id, error = %e, "Error processing stream message");
                            // Continue processing - don't break on individual errors
                        }
                    }
                }

                // Stream ended
                warn!(?chain, chain_id, "Protocol stream ended for chain");
            });

            chain_handles.push(handle);
        }

        // Update status to connected once we've spawned all streams
        {
            let mut state = self.state.write().await;
            state.status = ConnectionStatus::Connected;
        }

        // Record connection status metrics for all chains
        for chain_config in &self.chain_configs {
            let chain_id = chain_to_id(&chain_config.chain);
            record_connection_status(chain_id, true);
        }

        info!(
            chain_count = chain_handles.len(),
            "All chain streams started, waiting for streams to complete"
        );

        // Spawn a meta-task that waits for all chain handles
        let state = Arc::clone(&self.state);
        let handle = tokio::spawn(async move {
            // Wait for all chain streams to complete
            for (i, handle) in chain_handles.into_iter().enumerate() {
                match handle.await {
                    Ok(()) => info!(chain_index = i, "Chain stream task completed"),
                    Err(e) => error!(chain_index = i, error = %e, "Chain stream task panicked"),
                }
            }

            // All streams ended - update status
            let mut state_guard = state.write().await;
            state_guard.status = ConnectionStatus::Disconnected;
            warn!("All protocol streams have ended");
        });

        Ok(handle)
    }
}

/// Process a protocol update for a specific chain and update internal state.
///
/// This function uses chain-aware composite keys to prevent collisions between
/// pools on different chains with the same address.
///
/// It:
/// 1. Stores new protocol components (metadata) in the StateCache with chain prefix
/// 2. Stores/updates ProtocolSim states in the StateCache with chain prefix
/// 3. Removes deleted components from the StateCache
/// 4. Updates per-chain block number tracking
/// 5. Broadcasts update events to subscribers (for delta updates only)
///
/// The first message from the stream contains the FULL snapshot of all pools.
/// Subsequent messages contain only delta updates which are broadcast to subscribers.
///
/// # Delta Update Semantics
///
/// - `new_pairs`: New components added to tracking (broadcasts `UpdateType::New`)
/// - `states`: Updated simulation states (broadcasts `UpdateType::Modified` for existing)
/// - `removed_pairs`: Components removed from tracking (broadcasts `UpdateType::Removed`)
///
/// # Reorg Handling
///
/// The Tycho SDK handles blockchain reorgs automatically. When a reorg occurs:
/// - `removed_pairs` contains components invalidated by the reorg
/// - `states` contains the corrected state after reorg resolution
/// - This function applies these changes and broadcasts appropriate events
async fn process_update_for_chain(
    state: &Arc<RwLock<ManagerState>>,
    state_cache: &Arc<StateCache>,
    chain_id: u64,
    update: Update,
    is_first_message: bool,
    snapshot_start: Instant,
) {
    use crate::state::make_composite_key;

    // Track update processing time for metrics
    let update_start = Instant::now();

    // Update block number for this chain
    let block = update.block_number_or_timestamp;
    state_cache.set_latest_block_for_chain(chain_id, block);

    // Record block number metric
    set_latest_block_number(chain_id, block);

    {
        let mut state_guard = state.write().await;
        // Update to max block across chains
        if state_guard.latest_block.is_none_or(|b| block > b) {
            state_guard.latest_block = Some(block);
        }
    }

    // Track metrics
    let new_pairs_count = update.new_pairs.len();
    let updated_states_count = update.states.len();
    let removed_pairs_count = update.removed_pairs.len();

    // Collect IDs of new pairs for determining update type below
    let new_pair_ids: std::collections::HashSet<String> =
        update.new_pairs.keys().cloned().collect();

    // Store new protocol components (metadata) with chain-aware keys
    // These contain token info, protocol name, static attributes, etc.
    for (id, component) in update.new_pairs {
        let composite_key = make_composite_key(chain_id, &id);
        state_cache.insert_component(composite_key, component);
    }

    // Store/update ProtocolSim states with chain-aware keys
    // These are the actual simulation-ready state objects
    // For delta updates (not first message), broadcast events to subscribers
    if is_first_message {
        // Initial snapshot: bulk insert without broadcasting (too many events)
        for (id, sim_state) in update.states {
            let composite_key = make_composite_key(chain_id, &id);
            state_cache.insert_state(composite_key, sim_state);
        }
    } else {
        // Delta update: insert with event broadcasting
        for (id, sim_state) in update.states {
            // Determine if this is a new pool or an update to existing
            let is_new = new_pair_ids.contains(&id);
            state_cache.insert_state_with_event(chain_id, id, sim_state, block, is_new);
        }
    }

    // Remove deleted components and their states with chain-aware keys
    // This handles both TVL-based removals and reorg-based invalidations
    if is_first_message {
        // Initial snapshot: just remove without broadcasting
        for (id, _) in update.removed_pairs {
            let composite_key = make_composite_key(chain_id, &id);
            state_cache.remove(&composite_key);
        }
    } else {
        // Delta update: remove with event broadcasting
        for (id, _) in update.removed_pairs {
            state_cache.remove_with_event(chain_id, &id, block);
        }
    }

    // Mark snapshot complete for this chain after first message is processed
    if is_first_message {
        let duration = snapshot_start.elapsed();
        state_cache.mark_snapshot_complete_for_chain(chain_id, duration);

        // Record snapshot duration metric
        record_snapshot_duration(chain_id, duration.as_millis() as u64);

        // Record cache state counts
        set_cache_state_count(Some(chain_id), state_cache.state_count_for_chain(chain_id));

        info!(
            chain_id,
            block = block,
            new_components = new_pairs_count,
            states_loaded = updated_states_count,
            duration_ms = duration.as_millis(),
            "Initial state snapshot ingested for chain"
        );
    } else if new_pairs_count > 0 || removed_pairs_count > 0 || updated_states_count > 0 {
        // Record state update latency for delta updates
        let update_duration = update_start.elapsed();
        record_state_update_latency(chain_id, update_duration);

        // Update cache state count periodically
        set_cache_state_count(Some(chain_id), state_cache.state_count_for_chain(chain_id));

        debug!(
            chain_id,
            block = block,
            new_pairs = new_pairs_count,
            updated_states = updated_states_count,
            removed_pairs = removed_pairs_count,
            chain_states = state_cache.state_count_for_chain(chain_id),
            total_states = state_cache.state_count(),
            "Processed delta update for chain"
        );
    }
}

/// Convert a chain ID to a Chain enum.
fn chain_from_id(chain_id: u64) -> Result<Chain, TychoManagerError> {
    match chain_id {
        1 => Ok(Chain::Ethereum),
        8453 => Ok(Chain::Base),
        130 => Ok(Chain::Unichain),
        42161 => Ok(Chain::Arbitrum),
        324 => Ok(Chain::ZkSync),
        _ => Err(TychoManagerError::UnsupportedChain(chain_id)),
    }
}

/// Convert a Chain enum to a chain ID.
fn chain_to_id(chain: &Chain) -> u64 {
    match chain {
        Chain::Ethereum => 1,
        Chain::Base => 8453,
        Chain::Unichain => 130,
        Chain::Arbitrum => 42161,
        Chain::ZkSync => 324,
        _ => 0,
    }
}

/// Add a protocol to the stream builder.
///
/// This function maps protocol names to their corresponding state types.
fn add_protocol_to_stream(
    builder: ProtocolStreamBuilder,
    protocol: &str,
    filter: ComponentFilter,
) -> Result<ProtocolStreamBuilder, TychoManagerError> {
    let builder = match protocol {
        "uniswap_v2" | "sushiswap_v2" => builder.exchange::<UniswapV2State>(protocol, filter, None),
        "uniswap_v3" => builder.exchange::<UniswapV3State>(protocol, filter, None),
        // Add more protocols as needed
        _ => {
            warn!(protocol, "Unknown protocol, skipping");
            builder
        }
    };

    Ok(builder)
}

impl std::fmt::Debug for TychoManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TychoManager")
            .field("tycho_url", &self.tycho_url)
            .field("api_key", &"[REDACTED]")
            .field("chain_configs", &self.chain_configs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_from_id() {
        assert!(matches!(chain_from_id(1), Ok(Chain::Ethereum)));
        assert!(matches!(chain_from_id(8453), Ok(Chain::Base)));
        assert!(matches!(chain_from_id(130), Ok(Chain::Unichain)));
        assert!(matches!(
            chain_from_id(999),
            Err(TychoManagerError::UnsupportedChain(999))
        ));
    }

    #[test]
    fn test_chain_config_default_protocols() {
        let eth_config = ChainConfig::new(Chain::Ethereum);
        assert!(eth_config.protocols.contains(&"uniswap_v2".to_string()));
        assert!(eth_config.protocols.contains(&"uniswap_v3".to_string()));

        let base_config = ChainConfig::new(Chain::Base);
        assert!(base_config.protocols.contains(&"uniswap_v2".to_string()));
    }

    #[test]
    fn test_manager_state_default() {
        let state = ManagerState::default();
        assert_eq!(state.status, ConnectionStatus::Disconnected);
        assert!(state.tokens.is_empty());
        assert!(state.latest_block.is_none());
    }

    #[tokio::test]
    async fn test_tycho_manager_creation() {
        let config = Config {
            tycho_url: "wss://tycho-beta.propellerheads.xyz".to_string(),
            tycho_api_key: "test-key".to_string(),
            grpc_host: "0.0.0.0".to_string(),
            grpc_port: 50051,
            metrics_port: 9000,
            chain_ids: "1".to_string(),
            log_level: "info".to_string(),
        };

        let state_cache = Arc::new(StateCache::new());
        let manager = TychoManager::new(&config, state_cache).unwrap();
        assert_eq!(manager.tycho_url, "tycho-beta.propellerheads.xyz");
        assert_eq!(manager.chain_configs.len(), 1);
        assert!(matches!(manager.chain_configs[0].chain, Chain::Ethereum));
    }

    #[tokio::test]
    async fn test_connection_status() {
        let state_cache = Arc::new(StateCache::new());
        let manager = TychoManager::with_chain_configs(
            "test-url".to_string(),
            "test-key".to_string(),
            vec![ChainConfig::new(Chain::Ethereum)],
            state_cache,
        );

        assert_eq!(manager.status().await, ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_state_cache_integration() {
        let state_cache = Arc::new(StateCache::new());
        let manager = TychoManager::with_chain_configs(
            "test-url".to_string(),
            "test-key".to_string(),
            vec![ChainConfig::new(Chain::Ethereum)],
            Arc::clone(&state_cache),
        );

        // Initially empty
        assert_eq!(manager.state_count(), 0);
        assert_eq!(manager.component_count(), 0);
        assert!(!manager.is_snapshot_complete());

        // Verify the state cache is the same instance
        assert!(Arc::ptr_eq(&manager.state_cache, &state_cache));
    }

    #[test]
    fn test_chain_to_id() {
        assert_eq!(chain_to_id(&Chain::Ethereum), 1);
        assert_eq!(chain_to_id(&Chain::Base), 8453);
        assert_eq!(chain_to_id(&Chain::Unichain), 130);
        assert_eq!(chain_to_id(&Chain::Arbitrum), 42161);
        assert_eq!(chain_to_id(&Chain::ZkSync), 324);
    }

    #[tokio::test]
    async fn test_chain_partitioned_tokens() {
        let state_cache = Arc::new(StateCache::new());
        let manager = TychoManager::with_chain_configs(
            "test-url".to_string(),
            "test-key".to_string(),
            vec![ChainConfig::new(Chain::Ethereum)],
            state_cache,
        );

        // Initially, tokens should be empty for all chains
        let all_tokens = manager.tokens().await;
        assert!(all_tokens.is_empty());

        // Token count should be 0
        assert_eq!(manager.token_count().await, 0);

        // Tokens for specific chain should also be empty
        let eth_tokens = manager.tokens_for_chain(1).await;
        assert!(eth_tokens.is_empty());
    }
}
