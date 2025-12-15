//! State management module.
//!
//! Contains StateCache and TransientStateCache for managing protocol states.

mod cache;
mod transient;

pub use cache::{
    make_composite_key, parse_composite_key, CacheStats, PoolUpdateEvent, SharedState, StateCache,
    UpdateType,
};
pub use transient::{
    bytes_to_uuid, spawn_cleanup_task, uuid_to_bytes, CachedState, TransientStateCache,
    DEFAULT_TTL_SECS,
};
