//! Status handlers - Service health and status.
//!
//! This module implements:
//! - `get_status` - Get indexer sync status
//! - `health_check` - Service health check

use tracing::{debug, instrument};

use crate::bridge::BridgeService;
use crate::error::BridgeResult;
use crate::types::request::GetStatusRequest;
use crate::types::response::{GetStatusResponse, HealthCheckResponse, SyncStatus};

impl BridgeService {
    /// Get the current sync status of the indexer.
    #[instrument(skip(self))]
    pub fn get_status(&self, req: GetStatusRequest) -> BridgeResult<GetStatusResponse> {
        self.validate_chain(req.chain_id)?;

        // Get component count from cache
        let component_count = self.state_cache.component_count();

        // Get unique protocol systems
        let mut protocol_systems: Vec<String> = Vec::new();
        for component_id in self.state_cache.component_ids().iter() {
            if let Some(component) = self.state_cache.get_component(component_id) {
                let protocol = component.protocol_system.clone();
                if !protocol_systems.contains(&protocol) {
                    protocol_systems.push(protocol);
                }
            }
        }

        debug!(
            chain_id = req.chain_id,
            component_count,
            protocols = protocol_systems.len(),
            "Status check"
        );

        Ok(GetStatusResponse {
            sync_status: SyncStatus::Synced, // Assume synced if we have components
            latest_block: 0,                 // TODO: Track latest block from TychoManager
            synced_block: 0,                 // TODO: Track synced block
            component_count,
            protocol_systems,
        })
    }

    /// Health check - returns basic service health.
    pub fn health_check(&self) -> BridgeResult<HealthCheckResponse> {
        Ok(HealthCheckResponse {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            chains: self.chain_ids.clone(),
        })
    }

    /// Get the current sync status asynchronously.
    ///
    /// This version can access TychoManager state for more detailed status.
    #[instrument(skip(self))]
    pub async fn get_status_async(&self, req: GetStatusRequest) -> BridgeResult<GetStatusResponse> {
        self.validate_chain(req.chain_id)?;

        // Get component count from cache
        let component_count = self.state_cache.component_count();

        // Get token count from TychoManager
        let token_registry = self.tycho_manager.tokens().await;
        let token_count: usize = token_registry.values().map(|t| t.len()).sum();

        // Get unique protocol systems
        let mut protocol_systems: Vec<String> = Vec::new();
        for component_id in self.state_cache.component_ids().iter() {
            if let Some(component) = self.state_cache.get_component(component_id) {
                let protocol = component.protocol_system.clone();
                if !protocol_systems.contains(&protocol) {
                    protocol_systems.push(protocol);
                }
            }
        }

        debug!(
            chain_id = req.chain_id,
            component_count,
            token_count,
            protocols = protocol_systems.len(),
            "Async status check"
        );

        Ok(GetStatusResponse {
            sync_status: if component_count > 0 {
                SyncStatus::Synced
            } else {
                SyncStatus::Syncing
            },
            latest_block: 0, // TODO: Track latest block
            synced_block: 0, // TODO: Track synced block
            component_count,
            protocol_systems,
        })
    }
}
