//! Tycho gRPC Server - gRPC wrapper around tycho-core.
//!
//! This crate provides the gRPC server binary that exposes tycho-core
//! functionality via Protocol Buffers.
//!
//! # Architecture
//!
//! ```text
//! [gRPC Clients] ──> [GrpcService] ──> [BridgeService (tycho-core)]
//!                         │                      │
//!                    Proto types            Native types
//! ```
//!
//! The `GrpcService` is a thin adapter that:
//! 1. Receives proto request types from gRPC clients
//! 2. Converts them to tycho-core native types (via `convert` module)
//! 3. Calls the appropriate `BridgeService` handler
//! 4. Converts the native response back to proto types
//!
//! All business logic lives in tycho-core; this crate only handles transport.

pub mod convert;
pub mod proto;
pub mod service;

// Re-export key types for external use
pub use proto::{TychoBridge, TychoBridgeServer};
pub use service::GrpcService;
