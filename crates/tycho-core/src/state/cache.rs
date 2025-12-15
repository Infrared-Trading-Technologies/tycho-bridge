//! Thread-safe state cache for protocol simulation states.
//!
//! This module provides a high-performance, concurrent cache for storing protocol
//! simulation states from the Tycho indexer. The cache is designed for:
//!
//! 1. **Zero-Network Lookups**: All simulation queries execute against local memory
//! 2. **Thread Safety**: Safe concurrent access via DashMap
//! 3. **Efficient Updates**: Lock-free reads, minimal contention on writes
//! 4. **Multi-Chain Support**: States partitioned by chain ID to prevent collisions
//! 5. **Real-Time Broadcasting**: Update events are broadcast for streaming subscribers
//!
//! # Architecture
//!
//! ```text
//! [Tycho Stream] --> [StateCache] <-- [gRPC Handlers]
//!                         |
//!                    [DashMap<CompositeKey, Arc<RwLock<Box<dyn ProtocolSim>>>>]
//!                         |
//!            CompositeKey = "{chain_id}:{component_id}"
//!                         |
//!                    [broadcast::Sender<PoolUpdateEvent>] --> [gRPC StreamPoolUpdates]
//! ```
//!
//! # Multi-Chain Design
//!
//! To prevent collisions between pools on different chains with the same address,
//! all keys are prefixed with the chain ID using the format `{chain_id}:{component_id}`.
//! This ensures that Ethereum pool 0xABC and Base pool 0xABC are stored separately.
//!
//! # Real-Time Updates
//!
//! The cache maintains a broadcast channel for publishing state change events.
//! Subscribers (e.g., gRPC StreamPoolUpdates) can receive real-time notifications
//! about new, modified, or removed pools without polling.
//!
//! # SOLID Compliance
//!
//! - **Single Responsibility**: Only manages state storage and retrieval
//! - **Open/Closed**: New state types automatically supported via ProtocolSim trait
//! - **Interface Segregation**: Minimal public API surface
//! - **Dependency Inversion**: Depends on ProtocolSim trait, not concrete implementations

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::broadcast;
use tracing::{debug, info, trace};

use crate::domain::{ProtocolComponent, ProtocolSim};

/// Thread-safe wrapper for protocol simulation states.
///
/// We use Arc<RwLock<Box<dyn ProtocolSim>>> because:
/// 1. ProtocolSim is a trait object that needs to be heap-allocated (Box)
/// 2. States may be updated (swapped) atomically (RwLock for interior mutability)
/// 3. Multiple readers can access concurrently (Arc for shared ownership)
pub type SharedState = Arc<RwLock<Box<dyn ProtocolSim>>>;

/// Default capacity for the broadcast channel.
/// This determines how many updates can be buffered before slow receivers lag.
const BROADCAST_CHANNEL_CAPACITY: usize = 1024;

/// Type of update for a pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    /// New pool added to tracking (either newly created or meets filter criteria).
    New,
    /// Pool state was modified (liquidity change, swap, etc.).
    Modified,
    /// Pool removed from tracking (reorg, TVL drop, or deletion).
    Removed,
}

/// Event representing a single pool update for broadcasting.
///
/// This is published through the broadcast channel whenever a pool state changes.
/// Subscribers (like gRPC StreamPoolUpdates) receive these events in real-time.
#[derive(Debug, Clone)]
pub struct PoolUpdateEvent {
    /// Type of update (New, Modified, or Removed).
    pub update_type: UpdateType,
    /// Chain ID where this pool resides.
    pub chain_id: u64,
    /// Pool/component ID (the raw ID without chain prefix).
    pub pool_id: String,
    /// Block number at which this update occurred.
    pub block_number: u64,
    /// Timestamp when this event was created (for latency tracking).
    pub timestamp_ns: i64,
}

impl PoolUpdateEvent {
    /// Create a new pool update event with the current timestamp.
    pub fn new(update_type: UpdateType, chain_id: u64, pool_id: String, block_number: u64) -> Self {
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);

        Self {
            update_type,
            chain_id,
            pool_id,
            block_number,
            timestamp_ns,
        }
    }
}

/// Create a composite key for chain-partitioned storage.
///
/// Format: `{chain_id}:{component_id}`
/// Example: `1:0xabc123...` for Ethereum, `8453:0xabc123...` for Base
///
/// This prevents collisions between pools on different chains with the same address.
#[inline]
pub fn make_composite_key(chain_id: u64, component_id: &str) -> String {
    format!("{}:{}", chain_id, component_id)
}

/// Parse a composite key back into (chain_id, component_id).
///
/// Returns None if the key format is invalid.
#[inline]
pub fn parse_composite_key(composite_key: &str) -> Option<(u64, &str)> {
    let (chain_id_str, component_id) = composite_key.split_once(':')?;
    let chain_id = chain_id_str.parse().ok()?;
    Some((chain_id, component_id))
}

/// Thread-safe cache for protocol simulation states.
///
/// The cache stores `ProtocolSim` trait objects keyed by a composite key
/// (`{chain_id}:{component_id}`) to support multi-chain deployments without collisions.
/// This allows O(1) lookups for swap simulations.
///
/// # Multi-Chain Support
///
/// Keys are formatted as `{chain_id}:{component_id}` to ensure pools on different
/// chains with the same address are stored separately. Use [`make_composite_key`]
/// to create keys and [`parse_composite_key`] to parse them.
///
/// # Real-Time Updates
///
/// The cache maintains a broadcast channel for publishing update events.
/// Subscribe using [`subscribe`] to receive real-time notifications.
///
/// # Performance Characteristics
///
/// - Read: O(1) average, lock-free at DashMap level, RwLock read at state level
/// - Write: O(1) average, per-shard locking in DashMap, exclusive lock for state update
/// - Memory: ~200 bytes overhead per entry (plus state size)
///
/// # Example
///
/// ```ignore
/// use tycho_bridge::{StateCache, make_composite_key};
///
/// let cache = StateCache::new();
/// let key = make_composite_key(1, "0x...pool_address"); // Ethereum
///
/// // Subscribe to updates
/// let mut rx = cache.subscribe();
/// tokio::spawn(async move {
///     while let Ok(event) = rx.recv().await {
///         println!("Pool {} updated at block {}", event.pool_id, event.block_number);
///     }
/// });
///
/// // Get a state for simulation
/// if let Some(state) = cache.get_state(&key) {
///     let guard = state.read().unwrap();
///     let result = guard.get_amount_out(amount_in, &token_in, &token_out)?;
/// }
/// ```
pub struct StateCache {
    /// The underlying concurrent map storing simulation states.
    /// Key: Composite key in format `{chain_id}:{component_id}`
    /// Value: Thread-safe wrapper around the protocol simulation state
    states: DashMap<String, SharedState>,

    /// Protocol components (metadata) keyed by composite key.
    /// Contains token info, protocol name, static attributes, etc.
    components: DashMap<String, ProtocolComponent>,

    /// Per-chain block numbers for the last update.
    /// Key: chain_id, Value: block number
    chain_latest_blocks: DashMap<u64, u64>,

    /// Global latest block number (max across all chains).
    latest_block: AtomicU64,

    /// Total number of state updates received (for metrics).
    update_count: AtomicUsize,

    /// Total number of delta updates received (excludes initial snapshot).
    delta_update_count: AtomicUsize,

    /// Chains that have completed their initial snapshot.
    chains_with_snapshot: std::sync::RwLock<HashSet<u64>>,

    /// Time when the first chain's snapshot was completed.
    snapshot_completed_at: std::sync::RwLock<Option<Instant>>,

    /// Duration of the initial snapshot ingestion (for first chain).
    snapshot_duration: std::sync::RwLock<Option<Duration>>,

    /// Broadcast sender for real-time pool update events.
    /// Subscribers receive PoolUpdateEvent whenever pools are added, modified, or removed.
    update_sender: broadcast::Sender<PoolUpdateEvent>,
}

// Implement Debug manually since broadcast::Sender doesn't implement Debug
impl std::fmt::Debug for StateCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateCache")
            .field("states_count", &self.states.len())
            .field("components_count", &self.components.len())
            .field("latest_block", &self.latest_block.load(Ordering::Relaxed))
            .field("update_count", &self.update_count.load(Ordering::Relaxed))
            .field(
                "delta_update_count",
                &self.delta_update_count.load(Ordering::Relaxed),
            )
            .field("subscriber_count", &self.update_sender.receiver_count())
            .finish()
    }
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StateCache {
    /// Create a new empty state cache.
    pub fn new() -> Self {
        let (update_sender, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self {
            states: DashMap::new(),
            components: DashMap::new(),
            chain_latest_blocks: DashMap::new(),
            latest_block: AtomicU64::new(0),
            update_count: AtomicUsize::new(0),
            delta_update_count: AtomicUsize::new(0),
            chains_with_snapshot: std::sync::RwLock::new(HashSet::new()),
            snapshot_completed_at: std::sync::RwLock::new(None),
            snapshot_duration: std::sync::RwLock::new(None),
            update_sender,
        }
    }

    /// Create a new state cache with pre-allocated capacity.
    ///
    /// Use this when you know the approximate number of pools to reduce
    /// reallocations during initial snapshot ingestion.
    pub fn with_capacity(capacity: usize) -> Self {
        let (update_sender, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self {
            states: DashMap::with_capacity(capacity),
            components: DashMap::with_capacity(capacity),
            chain_latest_blocks: DashMap::new(),
            latest_block: AtomicU64::new(0),
            update_count: AtomicUsize::new(0),
            delta_update_count: AtomicUsize::new(0),
            chains_with_snapshot: std::sync::RwLock::new(HashSet::new()),
            snapshot_completed_at: std::sync::RwLock::new(None),
            snapshot_duration: std::sync::RwLock::new(None),
            update_sender,
        }
    }

    /// Subscribe to real-time pool update events.
    ///
    /// Returns a receiver that will receive [`PoolUpdateEvent`]s whenever
    /// pools are added, modified, or removed from the cache.
    ///
    /// # Notes
    ///
    /// - Subscribers may miss events if they process too slowly (lagging).
    /// - Each call creates a new independent subscriber.
    /// - The channel has a capacity of 1024 events before lagging occurs.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut rx = cache.subscribe();
    /// while let Ok(event) = rx.recv().await {
    ///     match event.update_type {
    ///         UpdateType::New => println!("New pool: {}", event.pool_id),
    ///         UpdateType::Modified => println!("Modified: {}", event.pool_id),
    ///         UpdateType::Removed => println!("Removed: {}", event.pool_id),
    ///     }
    /// }
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<PoolUpdateEvent> {
        self.update_sender.subscribe()
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.update_sender.receiver_count()
    }

    /// Broadcast a pool update event to all subscribers.
    ///
    /// This is called internally when pools are added, modified, or removed.
    /// Returns the number of receivers that received the event.
    fn broadcast_update(&self, event: PoolUpdateEvent) {
        // Only broadcast if there are subscribers (optimization)
        if self.update_sender.receiver_count() > 0 {
            // Log at trace level to avoid spamming during high-frequency updates
            trace!(
                update_type = ?event.update_type,
                chain_id = event.chain_id,
                pool_id = %event.pool_id,
                block = event.block_number,
                "Broadcasting pool update"
            );

            // Ignore send errors - they occur when no receivers are listening
            let _ = self.update_sender.send(event);
        }
    }

    /// Get a simulation state by composite key.
    ///
    /// The key should be in format `{chain_id}:{component_id}`. Use [`make_composite_key`]
    /// to create keys.
    ///
    /// Returns `None` if the component is not in the cache.
    /// The returned SharedState (Arc<RwLock<Box<dyn ProtocolSim>>>) allows
    /// concurrent reads via RwLock::read().
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key = make_composite_key(1, "0x...pool");
    /// if let Some(state) = cache.get_state(&key) {
    ///     let guard = state.read().unwrap();
    ///     let result = guard.get_amount_out(amount, &token_in, &token_out)?;
    /// }
    /// ```
    pub fn get_state(&self, composite_key: &str) -> Option<SharedState> {
        self.states.get(composite_key).map(|r| Arc::clone(&r))
    }

    /// Get a simulation state by chain ID and component ID.
    ///
    /// Convenience method that creates the composite key internally.
    pub fn get_state_for_chain(&self, chain_id: u64, component_id: &str) -> Option<SharedState> {
        let key = make_composite_key(chain_id, component_id);
        self.get_state(&key)
    }

    /// Get a protocol component (metadata) by composite key.
    pub fn get_component(&self, composite_key: &str) -> Option<ProtocolComponent> {
        self.components.get(composite_key).map(|r| r.clone())
    }

    /// Get a protocol component by chain ID and component ID.
    ///
    /// Convenience method that creates the composite key internally.
    pub fn get_component_for_chain(
        &self,
        chain_id: u64,
        component_id: &str,
    ) -> Option<ProtocolComponent> {
        let key = make_composite_key(chain_id, component_id);
        self.get_component(&key)
    }

    /// Check if a component exists in the cache by composite key.
    pub fn contains(&self, composite_key: &str) -> bool {
        self.states.contains_key(composite_key)
    }

    /// Check if a component exists by chain ID and component ID.
    pub fn contains_for_chain(&self, chain_id: u64, component_id: &str) -> bool {
        let key = make_composite_key(chain_id, component_id);
        self.contains(&key)
    }

    /// Insert or update a simulation state with chain awareness.
    ///
    /// This is the preferred method for inserting states. It automatically
    /// creates the composite key from chain_id and component_id.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain ID (e.g., 1 for Ethereum, 8453 for Base)
    /// * `component_id` - The component/pool ID (typically the pool address)
    /// * `state` - The protocol simulation state
    pub fn insert_state_for_chain(
        &self,
        chain_id: u64,
        component_id: String,
        state: Box<dyn ProtocolSim>,
    ) {
        let composite_key = make_composite_key(chain_id, &component_id);
        let shared_state = Arc::new(RwLock::new(state));
        self.states.insert(composite_key, shared_state);
        self.update_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Insert or update a simulation state by composite key.
    ///
    /// This is called by the stream processor when new states arrive.
    /// The Box is wrapped in Arc<RwLock<>> for thread-safe shared access.
    ///
    /// **Note**: Prefer [`insert_state_for_chain`] for new code.
    pub fn insert_state(&self, composite_key: String, state: Box<dyn ProtocolSim>) {
        let shared_state = Arc::new(RwLock::new(state));
        self.states.insert(composite_key, shared_state);
        self.update_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Insert or update a simulation state with delta update semantics.
    ///
    /// This method broadcasts an update event to subscribers, making it suitable
    /// for real-time streaming. Use this for delta updates (not initial snapshot).
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain ID
    /// * `component_id` - The component/pool ID
    /// * `state` - The protocol simulation state
    /// * `block_number` - Block number of this update
    /// * `is_new` - Whether this is a new component (vs update to existing)
    pub fn insert_state_with_event(
        &self,
        chain_id: u64,
        component_id: String,
        state: Box<dyn ProtocolSim>,
        block_number: u64,
        is_new: bool,
    ) {
        let composite_key = make_composite_key(chain_id, &component_id);
        let shared_state = Arc::new(RwLock::new(state));
        self.states.insert(composite_key, shared_state);
        self.update_count.fetch_add(1, Ordering::Relaxed);
        self.delta_update_count.fetch_add(1, Ordering::Relaxed);

        // Broadcast the update event
        let update_type = if is_new {
            UpdateType::New
        } else {
            UpdateType::Modified
        };
        let event = PoolUpdateEvent::new(update_type, chain_id, component_id, block_number);
        self.broadcast_update(event);
    }

    /// Insert or update a protocol component (metadata) with chain awareness.
    ///
    /// This is the preferred method for inserting components. It automatically
    /// creates the composite key from chain_id and component_id.
    pub fn insert_component_for_chain(
        &self,
        chain_id: u64,
        component_id: String,
        component: ProtocolComponent,
    ) {
        let composite_key = make_composite_key(chain_id, &component_id);
        self.components.insert(composite_key, component);
    }

    /// Insert or update a protocol component (metadata) by composite key.
    ///
    /// **Note**: Prefer [`insert_component_for_chain`] for new code.
    pub fn insert_component(&self, composite_key: String, component: ProtocolComponent) {
        self.components.insert(composite_key, component);
    }

    /// Remove a component and its state from the cache by composite key.
    ///
    /// Called when a component is removed from tracking (e.g., TVL drops below threshold).
    pub fn remove(&self, composite_key: &str) -> Option<SharedState> {
        self.components.remove(composite_key);
        self.states.remove(composite_key).map(|(_, v)| v)
    }

    /// Remove a component and its state by chain ID and component ID.
    pub fn remove_for_chain(&self, chain_id: u64, component_id: &str) -> Option<SharedState> {
        let key = make_composite_key(chain_id, component_id);
        self.remove(&key)
    }

    /// Remove a component with delta update semantics.
    ///
    /// This method broadcasts a removal event to subscribers, making it suitable
    /// for real-time streaming.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain ID
    /// * `component_id` - The component/pool ID
    /// * `block_number` - Block number of this removal
    pub fn remove_with_event(
        &self,
        chain_id: u64,
        component_id: &str,
        block_number: u64,
    ) -> Option<SharedState> {
        let key = make_composite_key(chain_id, component_id);
        let result = self.remove(&key);

        // Only broadcast if something was actually removed
        if result.is_some() {
            self.delta_update_count.fetch_add(1, Ordering::Relaxed);
            let event = PoolUpdateEvent::new(
                UpdateType::Removed,
                chain_id,
                component_id.to_string(),
                block_number,
            );
            self.broadcast_update(event);
        }

        result
    }

    /// Update the latest block number for a specific chain.
    pub fn set_latest_block_for_chain(&self, chain_id: u64, block: u64) {
        self.chain_latest_blocks.insert(chain_id, block);
        // Also update global latest block if this is the highest
        let current_max = self.latest_block.load(Ordering::Acquire);
        if block > current_max {
            self.latest_block.store(block, Ordering::Release);
        }
    }

    /// Update the latest block number (global, for backward compatibility).
    pub fn set_latest_block(&self, block: u64) {
        self.latest_block.store(block, Ordering::Release);
    }

    /// Get the latest block number for a specific chain.
    pub fn latest_block_for_chain(&self, chain_id: u64) -> Option<u64> {
        self.chain_latest_blocks.get(&chain_id).map(|r| *r)
    }

    /// Get the global latest block number (max across all chains).
    pub fn latest_block(&self) -> u64 {
        self.latest_block.load(Ordering::Acquire)
    }

    /// Get the total number of states in the cache.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Get the number of states for a specific chain.
    pub fn state_count_for_chain(&self, chain_id: u64) -> usize {
        let prefix = format!("{}:", chain_id);
        self.states
            .iter()
            .filter(|r| r.key().starts_with(&prefix))
            .count()
    }

    /// Get the total number of components in the cache.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Get the total number of state updates received.
    pub fn update_count(&self) -> usize {
        self.update_count.load(Ordering::Relaxed)
    }

    /// Get the total number of delta updates received (excludes initial snapshot).
    pub fn delta_update_count(&self) -> usize {
        self.delta_update_count.load(Ordering::Relaxed)
    }

    /// Get the number of chains that have completed their initial snapshot.
    pub fn chains_with_snapshot_count(&self) -> usize {
        self.chains_with_snapshot.read().unwrap().len()
    }

    /// Mark the initial snapshot as complete for a specific chain.
    ///
    /// This should be called after the first stream message is fully processed for each chain.
    /// The first chain to complete will set the global snapshot timing.
    pub fn mark_snapshot_complete_for_chain(&self, chain_id: u64, duration: Duration) {
        // Add chain to completed set
        {
            let mut chains = self.chains_with_snapshot.write().unwrap();
            chains.insert(chain_id);
        }

        // Only set global timing for the first chain to complete
        let mut completed = self.snapshot_completed_at.write().unwrap();
        if completed.is_none() {
            *completed = Some(Instant::now());

            let mut dur = self.snapshot_duration.write().unwrap();
            *dur = Some(duration);
        }

        info!(
            chain_id,
            duration_ms = duration.as_millis(),
            chain_state_count = self.state_count_for_chain(chain_id),
            total_state_count = self.state_count(),
            total_component_count = self.component_count(),
            "Chain snapshot complete"
        );
    }

    /// Mark the initial snapshot as complete and record timing (backward compatibility).
    ///
    /// This should be called after the first stream message is fully processed.
    pub fn mark_snapshot_complete(&self, duration: Duration) {
        let mut completed = self.snapshot_completed_at.write().unwrap();
        *completed = Some(Instant::now());

        let mut dur = self.snapshot_duration.write().unwrap();
        *dur = Some(duration);

        info!(
            duration_ms = duration.as_millis(),
            state_count = self.state_count(),
            component_count = self.component_count(),
            "Initial state snapshot complete"
        );
    }

    /// Check if a specific chain has completed its initial snapshot.
    pub fn is_chain_snapshot_complete(&self, chain_id: u64) -> bool {
        self.chains_with_snapshot
            .read()
            .unwrap()
            .contains(&chain_id)
    }

    /// Check if the initial snapshot has been completed.
    pub fn is_snapshot_complete(&self) -> bool {
        self.snapshot_completed_at.read().unwrap().is_some()
    }

    /// Get the duration of the initial snapshot ingestion.
    pub fn snapshot_duration(&self) -> Option<Duration> {
        *self.snapshot_duration.read().unwrap()
    }

    /// Iterate over all composite keys in the cache.
    ///
    /// Returns keys in format `{chain_id}:{component_id}`.
    /// Note: This is a snapshot in time; the cache may change during iteration.
    pub fn component_ids(&self) -> Vec<String> {
        self.states.iter().map(|r| r.key().clone()).collect()
    }

    /// Iterate over component IDs for a specific chain.
    ///
    /// Returns only the component_id portion (without chain prefix).
    pub fn component_ids_for_chain(&self, chain_id: u64) -> Vec<String> {
        let prefix = format!("{}:", chain_id);
        self.states
            .iter()
            .filter_map(|r| {
                let key = r.key();
                if key.starts_with(&prefix) {
                    // Strip the chain prefix to return just the component ID
                    Some(key[prefix.len()..].to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all components that contain a specific token.
    ///
    /// Useful for finding all pools that can trade a given token.
    pub fn components_with_token(&self, token_address: &str) -> Vec<ProtocolComponent> {
        self.components
            .iter()
            .filter_map(|r| {
                let component = r.value();
                // Check if any of the component's tokens match
                if component.tokens.iter().any(|t| {
                    // Token addresses are stored as Bytes, need to compare hex representation
                    format!("{:?}", t)
                        .to_lowercase()
                        .contains(&token_address.to_lowercase())
                }) {
                    Some(component.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get statistics about the cache state.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            state_count: self.state_count(),
            component_count: self.component_count(),
            latest_block: self.latest_block(),
            update_count: self.update_count(),
            delta_update_count: self.delta_update_count(),
            snapshot_complete: self.is_snapshot_complete(),
            snapshot_duration_ms: self.snapshot_duration().map(|d| d.as_millis() as u64),
            chains_with_snapshot: self.chains_with_snapshot_count(),
            subscriber_count: self.subscriber_count(),
        }
    }

    /// Clear all entries from the cache.
    ///
    /// Primarily used for testing or reconnection scenarios.
    /// Note: Does not close the broadcast channel - subscribers remain connected.
    pub fn clear(&self) {
        self.states.clear();
        self.components.clear();
        self.chain_latest_blocks.clear();
        self.latest_block.store(0, Ordering::Release);
        self.delta_update_count.store(0, Ordering::Release);
        {
            let mut chains = self.chains_with_snapshot.write().unwrap();
            chains.clear();
        }
        debug!("State cache cleared");
    }
}

/// Statistics about the state cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of simulation states in cache.
    pub state_count: usize,
    /// Number of protocol components in cache.
    pub component_count: usize,
    /// Latest block number (max across all chains).
    pub latest_block: u64,
    /// Total number of state updates received (including snapshot).
    pub update_count: usize,
    /// Number of delta updates received (excludes initial snapshot).
    pub delta_update_count: usize,
    /// Whether any chain has completed its initial snapshot.
    pub snapshot_complete: bool,
    /// Duration of first snapshot ingestion in milliseconds.
    pub snapshot_duration_ms: Option<u64>,
    /// Number of chains that have completed their snapshot.
    pub chains_with_snapshot: usize,
    /// Number of active broadcast subscribers.
    pub subscriber_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_key() {
        let key = make_composite_key(1, "0xabc123");
        assert_eq!(key, "1:0xabc123");

        let key = make_composite_key(8453, "0xdef456");
        assert_eq!(key, "8453:0xdef456");
    }

    #[test]
    fn test_parse_composite_key() {
        let (chain_id, component_id) = parse_composite_key("1:0xabc123").unwrap();
        assert_eq!(chain_id, 1);
        assert_eq!(component_id, "0xabc123");

        let (chain_id, component_id) = parse_composite_key("8453:0xdef456").unwrap();
        assert_eq!(chain_id, 8453);
        assert_eq!(component_id, "0xdef456");

        // Invalid format
        assert!(parse_composite_key("invalid").is_none());
        assert!(parse_composite_key("notanumber:0xabc").is_none());
    }

    #[test]
    fn test_cache_creation() {
        let cache = StateCache::new();
        assert_eq!(cache.state_count(), 0);
        assert_eq!(cache.component_count(), 0);
        assert_eq!(cache.latest_block(), 0);
        assert!(!cache.is_snapshot_complete());
        assert_eq!(cache.chains_with_snapshot_count(), 0);
    }

    #[test]
    fn test_cache_with_capacity() {
        let cache = StateCache::with_capacity(1000);
        assert_eq!(cache.state_count(), 0);
    }

    #[test]
    fn test_block_number_updates() {
        let cache = StateCache::new();
        cache.set_latest_block(12345);
        assert_eq!(cache.latest_block(), 12345);

        cache.set_latest_block(12346);
        assert_eq!(cache.latest_block(), 12346);
    }

    #[test]
    fn test_per_chain_block_numbers() {
        let cache = StateCache::new();

        // Set block for Ethereum
        cache.set_latest_block_for_chain(1, 20000000);
        assert_eq!(cache.latest_block_for_chain(1), Some(20000000));
        assert_eq!(cache.latest_block(), 20000000);

        // Set block for Base (higher)
        cache.set_latest_block_for_chain(8453, 30000000);
        assert_eq!(cache.latest_block_for_chain(8453), Some(30000000));
        assert_eq!(cache.latest_block(), 30000000); // Global max updated

        // Set another block for Ethereum (lower than Base)
        cache.set_latest_block_for_chain(1, 20000001);
        assert_eq!(cache.latest_block_for_chain(1), Some(20000001));
        assert_eq!(cache.latest_block(), 30000000); // Global max unchanged
    }

    #[test]
    fn test_snapshot_complete() {
        let cache = StateCache::new();
        assert!(!cache.is_snapshot_complete());
        assert!(cache.snapshot_duration().is_none());

        cache.mark_snapshot_complete(Duration::from_millis(500));

        assert!(cache.is_snapshot_complete());
        assert_eq!(cache.snapshot_duration(), Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_per_chain_snapshot_complete() {
        let cache = StateCache::new();

        // Mark Ethereum snapshot complete
        cache.mark_snapshot_complete_for_chain(1, Duration::from_millis(100));
        assert!(cache.is_chain_snapshot_complete(1));
        assert!(!cache.is_chain_snapshot_complete(8453));
        assert!(cache.is_snapshot_complete()); // Global is now true
        assert_eq!(cache.chains_with_snapshot_count(), 1);

        // Mark Base snapshot complete
        cache.mark_snapshot_complete_for_chain(8453, Duration::from_millis(200));
        assert!(cache.is_chain_snapshot_complete(8453));
        assert_eq!(cache.chains_with_snapshot_count(), 2);

        // Duration should still be from first chain
        assert_eq!(cache.snapshot_duration(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_stats() {
        let cache = StateCache::new();
        cache.set_latest_block(100);

        let stats = cache.stats();
        assert_eq!(stats.state_count, 0);
        assert_eq!(stats.latest_block, 100);
        assert!(!stats.snapshot_complete);
        assert_eq!(stats.chains_with_snapshot, 0);
    }

    #[test]
    fn test_clear() {
        let cache = StateCache::new();
        cache.set_latest_block(100);
        cache.set_latest_block_for_chain(1, 100);
        cache.mark_snapshot_complete_for_chain(1, Duration::from_millis(50));

        cache.clear();

        assert_eq!(cache.latest_block(), 0);
        assert_eq!(cache.state_count(), 0);
        assert!(cache.latest_block_for_chain(1).is_none());
        assert_eq!(cache.chains_with_snapshot_count(), 0);
    }

    #[test]
    fn test_contains() {
        let cache = StateCache::new();
        let key = make_composite_key(1, "0x123");
        assert!(!cache.contains(&key));
        assert!(!cache.contains_for_chain(1, "0x123"));
    }

    #[test]
    fn test_remove_nonexistent() {
        let cache = StateCache::new();
        let key = make_composite_key(1, "0x123");
        assert!(cache.remove(&key).is_none());
        assert!(cache.remove_for_chain(1, "0x123").is_none());
    }

    #[test]
    fn test_component_ids_empty() {
        let cache = StateCache::new();
        assert!(cache.component_ids().is_empty());
    }

    #[test]
    fn test_subscriber_count() {
        let cache = StateCache::new();
        assert_eq!(cache.subscriber_count(), 0);

        // Subscribe
        let _rx1 = cache.subscribe();
        assert_eq!(cache.subscriber_count(), 1);

        let _rx2 = cache.subscribe();
        assert_eq!(cache.subscriber_count(), 2);

        // Dropping subscriber decreases count
        drop(_rx1);
        assert_eq!(cache.subscriber_count(), 1);
    }

    #[test]
    fn test_update_type_enum() {
        assert_eq!(UpdateType::New, UpdateType::New);
        assert_ne!(UpdateType::New, UpdateType::Modified);
        assert_ne!(UpdateType::Modified, UpdateType::Removed);
    }

    #[test]
    fn test_pool_update_event_creation() {
        let event = PoolUpdateEvent::new(UpdateType::New, 1, "0xabc".to_string(), 12345);
        assert_eq!(event.update_type, UpdateType::New);
        assert_eq!(event.chain_id, 1);
        assert_eq!(event.pool_id, "0xabc");
        assert_eq!(event.block_number, 12345);
        assert!(event.timestamp_ns > 0);
    }

    #[test]
    fn test_stats_includes_new_fields() {
        let cache = StateCache::new();
        let _rx = cache.subscribe();

        let stats = cache.stats();
        assert_eq!(stats.delta_update_count, 0);
        assert_eq!(stats.subscriber_count, 1);
    }

    #[test]
    fn test_delta_update_count_initial() {
        let cache = StateCache::new();
        // Verify delta update count starts at 0
        assert_eq!(cache.delta_update_count(), 0);

        // Regular insert should not increment delta count
        // (we can't easily create a mock ProtocolSim without alloy dependency)
        // but we can verify the counter accessor works
        let stats = cache.stats();
        assert_eq!(stats.delta_update_count, 0);
    }
}
