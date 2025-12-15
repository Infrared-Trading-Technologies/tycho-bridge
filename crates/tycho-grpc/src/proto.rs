//! Proto type definitions and re-exports for the gRPC service.
//!
//! This module contains the generated protobuf types and re-exports
//! commonly used types for external consumption.

/// Generated protobuf types for the Tycho Bridge v1 API.
mod generated {
    tonic::include_proto!("tycho.bridge.v1");
}

// Re-export everything from generated at this level for easier access
pub use generated::*;

// Re-export server and client types at the crate level
pub use generated::tycho_bridge_server::{TychoBridge, TychoBridgeServer};

#[allow(unused_imports)]
pub use generated::tycho_bridge_client::TychoBridgeClient;

// Re-export nested modules for result types
pub use generated::get_amount_out_result;

// Type aliases for clarity
pub type ProtoEncoderType = generated::EncoderType;
pub type ProtoNativeAction = generated::NativeAction;
pub type ProtoUserTransferType = generated::UserTransferType;
pub type ProtoComponent = generated::ProtocolComponent;
pub type ProtoToken = generated::Token;
pub type ProtoSwap = generated::Swap;
pub type ProtoSolution = generated::Solution;
pub type ProtoPermit = generated::Permit;
