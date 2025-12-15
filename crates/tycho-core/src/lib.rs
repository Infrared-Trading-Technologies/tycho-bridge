//! Tycho Core - Transport-agnostic business logic for Tycho bridge.
//!
//! This crate contains the core functionality for interacting with the Tycho indexer:
//! - State management (StateCache, TransientStateCache)
//! - Connection management (TychoManager)
//! - Simulation handlers (get_amount_out, spot_price, etc.)
//! - Execution encoding
//!
//! This crate has NO transport dependencies (no gRPC, no FFI).
//! Transport layers (tycho-grpc, tycho-ffi) depend on this crate.

pub mod bridge;
pub mod config;
pub mod domain;
pub mod error;
pub mod handlers;
pub mod helpers;
pub mod manager;
pub mod metrics;
pub mod state;
pub mod types;

pub use bridge::BridgeService;
pub use config::{Config, ConfigError};
pub use error::BridgeError;
pub use manager::TychoManager;
pub use state::{make_composite_key, StateCache, TransientStateCache};
