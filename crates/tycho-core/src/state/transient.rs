//! Transient state cache for simulation state chaining.
//!
//! This module provides a temporary cache for storing modified `ProtocolSim` states
//! that result from swap simulations. This enables state chaining - where the output
//! state of one simulation becomes the input state of the next.
//!
//! # Use Case
//!
//! When simulating a swap via `GetAmountOut`, the Tycho SDK returns a `new_state`
//! representing the pool after the swap. To chain simulations (e.g., for arbitrage
//! testing), we need to pass this modified state to subsequent calls.
//!
//! Since `Box<dyn ProtocolSim>` cannot be serialized across gRPC, we:
//! 1. Store the modified state in memory with a UUID key
//! 2. Return the UUID to the client as `new_pool_state` bytes
//! 3. Accept the UUID in `pool_state_override` to retrieve the cached state
//!
//! # Lifecycle
//!
//! - States are stored with a configurable TTL (default: 60 seconds)
//! - A background task periodically cleans up expired entries
//! - On service restart, all cached states are lost (clients must handle gracefully)
//!
//! # Architecture
//!
//! ```text
//! [GetAmountOut] --> [ProtocolSim.get_amount_out()]
//!                           |
//!                           v
//!                    [new_state: Box<dyn ProtocolSim>]
//!                           |
//!                           v
//!                    [TransientStateCache]
//!                           |
//!                           +---> Store with UUID key
//!                           |
//!                           v
//!                    [Return UUID as bytes]
//! ```
//!
//! # SOLID Compliance
//!
//! - **Single Responsibility**: Only manages transient state storage
//! - **Open/Closed**: Works with any ProtocolSim implementation
//! - **Dependency Inversion**: Depends on ProtocolSim trait, not concrete types
//!
//! # 12-Factor Compliance
//!
//! - **Config**: TTL configurable via `TRANSIENT_STATE_TTL_SECS` environment variable
//! - **Stateless**: This is ephemeral cache; authoritative state is from Tycho stream
//! - **Logs**: All cache operations logged via tracing

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::domain::ProtocolSim;

/// Default TTL for cached states (60 seconds).
pub const DEFAULT_TTL_SECS: u64 = 60;

/// Default cleanup interval (30 seconds).
const CLEANUP_INTERVAL_SECS: u64 = 30;

/// Thread-safe wrapper for a cached protocol state.
pub type CachedState = Arc<RwLock<Box<dyn ProtocolSim>>>;

/// Entry in the transient cache with expiration tracking.
struct CacheEntry {
    /// The cached protocol state.
    state: CachedState,
    /// When this entry was created.
    created_at: Instant,
    /// Chain ID for logging/debugging.
    chain_id: u64,
    /// Component ID for logging/debugging.
    component_id: String,
}

/// Transient cache for storing modified simulation states.
///
/// This cache enables state chaining across gRPC calls by storing
/// `Box<dyn ProtocolSim>` states with UUID keys.
pub struct TransientStateCache {
    /// The underlying storage.
    entries: DashMap<Uuid, CacheEntry>,
    /// Time-to-live for cached entries.
    ttl: Duration,
}

impl TransientStateCache {
    /// Creates a new transient state cache with the specified TTL.
    ///
    /// # Arguments
    ///
    /// * `ttl_secs` - Time-to-live in seconds for cached entries
    pub fn new(ttl_secs: u64) -> Self {
        info!(ttl_secs, "Creating transient state cache");
        Self {
            entries: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Creates a new transient state cache with the default TTL.
    pub fn with_default_ttl() -> Self {
        Self::new(DEFAULT_TTL_SECS)
    }

    /// Stores a state and returns its UUID.
    ///
    /// # Arguments
    ///
    /// * `state` - The protocol state to cache
    /// * `chain_id` - Chain ID for logging
    /// * `component_id` - Component ID for logging
    ///
    /// # Returns
    ///
    /// A UUID that can be used to retrieve the state later.
    pub fn store(&self, state: Box<dyn ProtocolSim>, chain_id: u64, component_id: &str) -> Uuid {
        let id = Uuid::new_v4();
        let entry = CacheEntry {
            state: Arc::new(RwLock::new(state)),
            created_at: Instant::now(),
            chain_id,
            component_id: component_id.to_string(),
        };

        self.entries.insert(id, entry);

        debug!(
            state_id = %id,
            chain_id,
            component_id,
            cache_size = self.entries.len(),
            "Stored transient state"
        );

        id
    }

    /// Retrieves a cached state by UUID.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID returned from `store()`
    ///
    /// # Returns
    ///
    /// The cached state if found and not expired, or `None` otherwise.
    pub fn get(&self, id: &Uuid) -> Option<CachedState> {
        let entry = self.entries.get(id)?;

        // Check if expired
        if entry.created_at.elapsed() > self.ttl {
            trace!(state_id = %id, "State expired, removing");
            drop(entry); // Release the read lock before removing
            self.entries.remove(id);
            return None;
        }

        trace!(
            state_id = %id,
            chain_id = entry.chain_id,
            component_id = %entry.component_id,
            age_ms = entry.created_at.elapsed().as_millis(),
            "Retrieved transient state"
        );

        Some(Arc::clone(&entry.state))
    }

    /// Removes a cached state by UUID.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID to remove
    ///
    /// # Returns
    ///
    /// `true` if the state was found and removed, `false` otherwise.
    pub fn remove(&self, id: &Uuid) -> bool {
        let removed = self.entries.remove(id).is_some();
        if removed {
            debug!(state_id = %id, "Removed transient state");
        }
        removed
    }

    /// Returns the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Removes all expired entries.
    ///
    /// This is called periodically by the cleanup task.
    ///
    /// # Returns
    ///
    /// The number of entries removed.
    pub fn cleanup_expired(&self) -> usize {
        let before = self.entries.len();

        self.entries.retain(|id, entry| {
            let expired = entry.created_at.elapsed() > self.ttl;
            if expired {
                trace!(
                    state_id = %id,
                    chain_id = entry.chain_id,
                    component_id = %entry.component_id,
                    "Cleaning up expired state"
                );
            }
            !expired
        });

        let removed = before - self.entries.len();
        if removed > 0 {
            debug!(
                removed,
                remaining = self.entries.len(),
                "Cleaned up expired states"
            );
        }
        removed
    }

    /// Clears all cached entries.
    pub fn clear(&self) {
        let count = self.entries.len();
        self.entries.clear();
        info!(removed = count, "Cleared transient state cache");
    }

    /// Returns the configured TTL.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}

impl Default for TransientStateCache {
    fn default() -> Self {
        Self::with_default_ttl()
    }
}

/// Converts a UUID to bytes for gRPC transport.
///
/// The UUID is serialized as 16 bytes (128 bits).
pub fn uuid_to_bytes(id: &Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

/// Parses bytes into a UUID.
///
/// # Arguments
///
/// * `bytes` - 16 bytes representing a UUID
///
/// # Returns
///
/// The parsed UUID, or `None` if the bytes are invalid.
pub fn bytes_to_uuid(bytes: &[u8]) -> Option<Uuid> {
    if bytes.len() != 16 {
        warn!(len = bytes.len(), "Invalid UUID bytes length (expected 16)");
        return None;
    }

    let bytes_array: [u8; 16] = bytes.try_into().ok()?;
    Some(Uuid::from_bytes(bytes_array))
}

/// Spawns a background task that periodically cleans up expired entries.
///
/// # Arguments
///
/// * `cache` - The cache to clean up (must be wrapped in Arc)
///
/// # Returns
///
/// A handle to the spawned task.
pub fn spawn_cleanup_task(cache: Arc<TransientStateCache>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = Duration::from_secs(CLEANUP_INTERVAL_SECS);
        info!(
            interval_secs = CLEANUP_INTERVAL_SECS,
            ttl_secs = cache.ttl().as_secs(),
            "Starting transient state cleanup task"
        );

        loop {
            tokio::time::sleep(interval).await;
            cache.cleanup_expired();
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use num_bigint::BigUint;
    use tycho_simulation::tycho_common::dto::ProtocolStateDelta;
    use tycho_simulation::tycho_common::models::token::Token;
    use tycho_simulation::tycho_common::simulation::errors::{SimulationError, TransitionError};
    use tycho_simulation::tycho_common::simulation::protocol_sim::{Balances, GetAmountOutResult};
    use tycho_simulation::tycho_common::Bytes;

    /// Mock ProtocolSim for testing.
    #[derive(Clone, Debug)]
    struct MockState {
        reserve0: u64,
        reserve1: u64,
    }

    impl ProtocolSim for MockState {
        fn fee(&self) -> f64 {
            0.003
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
            // Simple constant product formula
            let amount_in_u64: u64 = amount_in.try_into().unwrap_or(0);
            let amount_out = (self.reserve1 * amount_in_u64) / (self.reserve0 + amount_in_u64);

            let new_state = Box::new(MockState {
                reserve0: self.reserve0 + amount_in_u64,
                reserve1: self.reserve1 - amount_out,
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
                .downcast_ref::<MockState>()
                .map(|o| o.reserve0 == self.reserve0 && o.reserve1 == self.reserve1)
                .unwrap_or(false)
        }
    }

    #[test]
    fn test_store_and_retrieve() {
        let cache = TransientStateCache::new(60);
        let state = Box::new(MockState {
            reserve0: 1000,
            reserve1: 2000,
        });

        let id = cache.store(state, 1, "test-pool");

        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_get_nonexistent() {
        let cache = TransientStateCache::new(60);
        let random_id = Uuid::new_v4();

        assert!(cache.get(&random_id).is_none());
    }

    #[test]
    fn test_remove() {
        let cache = TransientStateCache::new(60);
        let state = Box::new(MockState {
            reserve0: 1000,
            reserve1: 2000,
        });

        let id = cache.store(state, 1, "test-pool");
        assert_eq!(cache.len(), 1);

        assert!(cache.remove(&id));
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&id).is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let cache = TransientStateCache::new(60);
        let random_id = Uuid::new_v4();

        assert!(!cache.remove(&random_id));
    }

    #[test]
    fn test_clear() {
        let cache = TransientStateCache::new(60);

        for i in 0..5 {
            let state = Box::new(MockState {
                reserve0: i * 100,
                reserve1: i * 200,
            });
            cache.store(state, 1, &format!("pool-{}", i));
        }

        assert_eq!(cache.len(), 5);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_expired_entry_not_returned() {
        // Use 0-second TTL for immediate expiration
        let cache = TransientStateCache::new(0);
        let state = Box::new(MockState {
            reserve0: 1000,
            reserve1: 2000,
        });

        let id = cache.store(state, 1, "test-pool");

        // Wait a tiny bit to ensure expiration
        std::thread::sleep(Duration::from_millis(10));

        // Should return None because it's expired
        assert!(cache.get(&id).is_none());
        // Entry should be removed on get
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cleanup_expired() {
        let cache = TransientStateCache::new(0); // 0-second TTL

        for i in 0..5 {
            let state = Box::new(MockState {
                reserve0: i * 100,
                reserve1: i * 200,
            });
            cache.store(state, 1, &format!("pool-{}", i));
        }

        assert_eq!(cache.len(), 5);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(10));

        let removed = cache.cleanup_expired();
        assert_eq!(removed, 5);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_uuid_round_trip() {
        let original = Uuid::new_v4();
        let bytes = uuid_to_bytes(&original);
        let recovered = bytes_to_uuid(&bytes);

        assert_eq!(Some(original), recovered);
    }

    #[test]
    fn test_bytes_to_uuid_invalid_length() {
        let short = vec![0u8; 8];
        assert!(bytes_to_uuid(&short).is_none());

        let long = vec![0u8; 32];
        assert!(bytes_to_uuid(&long).is_none());
    }

    #[test]
    fn test_default_ttl() {
        let cache = TransientStateCache::default();
        assert_eq!(cache.ttl().as_secs(), DEFAULT_TTL_SECS);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let cache = Arc::new(TransientStateCache::new(60));
        let mut handles = vec![];

        // Spawn multiple tasks that store and retrieve
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = tokio::spawn(async move {
                let state = Box::new(MockState {
                    reserve0: i * 100,
                    reserve1: i * 200,
                });
                let id = cache_clone.store(state, 1, &format!("pool-{}", i));

                // Retrieve immediately
                let retrieved = cache_clone.get(&id);
                assert!(retrieved.is_some());

                id
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(cache.len(), 10);
    }
}
