//! BridgeService - Transport-agnostic bridge implementation.
//!
//! This is the main entry point for all bridge operations.
//! Transport layers (gRPC, FFI) wrap this service.

use std::sync::Arc;
use std::time::Instant;

use crate::config::Config;
use crate::error::{BridgeError, BridgeResult};
use crate::manager::TychoManager;
use crate::state::{StateCache, TransientStateCache};

/// Core bridge service that provides all Tycho functionality.
///
/// This struct is transport-agnostic and contains the business logic
/// for all bridge operations. Transport layers (gRPC, FFI, HTTP)
/// wrap this service and convert between their types and the native types.
///
/// # Thread Safety
///
/// BridgeService is designed for concurrent access. All internal state
/// is protected by appropriate synchronization primitives.
#[derive(Clone)]
pub struct BridgeService {
    /// Thread-safe state cache containing protocol simulation states.
    pub(crate) state_cache: Arc<StateCache>,

    /// Tycho manager for connection status and token access.
    pub(crate) tycho_manager: Arc<TychoManager>,

    /// Transient cache for storing modified simulation states.
    pub(crate) transient_state_cache: Arc<TransientStateCache>,

    /// Configured chain IDs for this service instance.
    pub(crate) chain_ids: Vec<u64>,

    /// Tycho Indexer URL (for status reporting).
    pub(crate) indexer_url: String,

    /// Service start time for uptime calculation.
    pub(crate) start_time: Instant,
}

impl BridgeService {
    /// Create a new BridgeService with injected dependencies.
    ///
    /// # Arguments
    ///
    /// * `state_cache` - Thread-safe cache for protocol simulation states
    /// * `tycho_manager` - Manager for Tycho connection and tokens
    /// * `transient_state_cache` - Cache for storing modified states across calls
    /// * `chain_ids` - List of chain IDs this service is configured for
    /// * `indexer_url` - Tycho Indexer URL for status reporting
    pub fn new(
        state_cache: Arc<StateCache>,
        tycho_manager: Arc<TychoManager>,
        transient_state_cache: Arc<TransientStateCache>,
        chain_ids: Vec<u64>,
        indexer_url: String,
    ) -> Self {
        Self {
            state_cache,
            tycho_manager,
            transient_state_cache,
            chain_ids,
            indexer_url,
            start_time: Instant::now(),
        }
    }

    /// Create a BridgeService from configuration.
    ///
    /// This is a convenience method that initializes all dependencies.
    pub async fn from_config(config: &Config) -> BridgeResult<Self> {
        let chain_ids = config
            .parsed_chain_ids()
            .map_err(|e| BridgeError::ConfigError {
                message: e.to_string(),
            })?;

        let state_cache = Arc::new(StateCache::new());
        let transient_cache = Arc::new(TransientStateCache::with_default_ttl());

        let tycho_manager = TychoManager::new(config, Arc::clone(&state_cache)).map_err(|e| {
            BridgeError::ConfigError {
                message: e.to_string(),
            }
        })?;

        Ok(Self::new(
            state_cache,
            Arc::new(tycho_manager),
            transient_cache,
            chain_ids,
            config.tycho_url.clone(),
        ))
    }

    /// Get the state cache.
    pub fn state_cache(&self) -> &Arc<StateCache> {
        &self.state_cache
    }

    /// Get the Tycho manager.
    pub fn tycho_manager(&self) -> &Arc<TychoManager> {
        &self.tycho_manager
    }

    /// Get the transient state cache.
    pub fn transient_state_cache(&self) -> &Arc<TransientStateCache> {
        &self.transient_state_cache
    }

    /// Get configured chain IDs.
    pub fn chain_ids(&self) -> &[u64] {
        &self.chain_ids
    }

    /// Get service uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Check if a chain is supported.
    pub fn is_chain_supported(&self, chain_id: u64) -> bool {
        self.chain_ids.contains(&chain_id)
    }

    /// Validate that a chain is supported, returning error if not.
    pub fn validate_chain(&self, chain_id: u64) -> BridgeResult<()> {
        if !self.is_chain_supported(chain_id) {
            return Err(BridgeError::ChainNotSupported { chain_id });
        }
        Ok(())
    }
}

impl std::fmt::Debug for BridgeService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BridgeService")
            .field("chain_ids", &self.chain_ids)
            .field("indexer_url", &self.indexer_url)
            .field("uptime_seconds", &self.uptime_seconds())
            .finish()
    }
}
