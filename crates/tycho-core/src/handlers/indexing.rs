//! Indexing handlers - Component and Token operations.
//!
//! This module implements:
//! - `get_components` - Query protocol components
//! - `get_tokens` - List all tokens
//! - `get_component` - Get single component by ID

use tracing::{debug, info, instrument};

use crate::bridge::BridgeService;
use crate::error::BridgeResult;
use crate::helpers::apply_pagination;
use crate::state::make_composite_key;
use crate::types::component::evm_chain_to_u64;
use crate::types::request::{GetComponentRequest, GetComponentsRequest, GetTokensRequest};
use crate::types::response::{
    ComponentInfo, GetComponentResponse, GetComponentsResponse, GetTokensResponse, TokenInfo,
};

impl BridgeService {
    /// Get protocol components matching filters.
    #[instrument(skip(self))]
    pub fn get_components(&self, req: GetComponentsRequest) -> BridgeResult<GetComponentsResponse> {
        self.validate_chain(req.chain_id)?;

        let (page_size, offset) = apply_pagination(
            req.limit
                .map(|l| ((offset_for_limit(l)) + 1) as u32)
                .unwrap_or(1),
            req.limit.unwrap_or(100),
        );

        // Get all component IDs from cache
        let component_ids = self.state_cache.component_ids();

        let mut matching_components: Vec<ComponentInfo> = Vec::new();

        for component_id in component_ids.iter() {
            if let Some(component) = self.state_cache.get_component(component_id) {
                // Apply chain filter
                let comp_chain_id = evm_chain_to_u64(&component.chain);
                if req.chain_id != 0 && comp_chain_id != req.chain_id {
                    continue;
                }

                // Apply protocol filter
                if let Some(ref protocol_type) = req.protocol_type {
                    if !component
                        .protocol_system
                        .to_lowercase()
                        .contains(&protocol_type.to_lowercase())
                    {
                        continue;
                    }
                }

                // Apply component ID filter
                if !req.component_ids.is_empty() {
                    let comp_id_hex = format!("0x{}", hex::encode(&component.id));
                    if !req
                        .component_ids
                        .iter()
                        .any(|id| id.to_lowercase() == comp_id_hex.to_lowercase())
                    {
                        continue;
                    }
                }

                // TODO: Apply min_tvl filter when TVL tracking is implemented

                // Convert to response type
                let comp_info = ComponentInfo {
                    id: format!("0x{}", hex::encode(&component.id)),
                    protocol_system: component.protocol_system.clone(),
                    protocol_type_name: component.protocol_type_name.clone(),
                    chain_id: comp_chain_id,
                    tokens: component
                        .tokens
                        .iter()
                        .map(|t| format!("0x{}", hex::encode(&t.address)))
                        .collect(),
                    tvl_usd: None, // TODO: Track TVL
                    created_at: component.created_at.and_utc().timestamp(),
                };

                matching_components.push(comp_info);
            }
        }

        // Apply pagination
        let total = matching_components.len();
        let paginated: Vec<ComponentInfo> = matching_components
            .into_iter()
            .skip(offset)
            .take(page_size)
            .collect();

        debug!(
            component_count = paginated.len(),
            total, "GetComponents response"
        );

        Ok(GetComponentsResponse {
            components: paginated,
        })
    }

    /// Get all tokens from the indexer.
    ///
    /// Note: This method is synchronous but accesses async TychoManager data.
    /// For async access, use `get_tokens_async`.
    #[instrument(skip(self))]
    pub fn get_tokens(&self, _req: GetTokensRequest) -> BridgeResult<GetTokensResponse> {
        // For synchronous access, we can't get tokens from TychoManager
        // This is a limitation of the current design - need to use async version
        // or have tokens cached synchronously

        // Return empty for now - the async version will be used by gRPC
        Ok(GetTokensResponse { tokens: vec![] })
    }

    /// Get a single component by ID.
    #[instrument(skip(self), fields(component_id = %req.component_id))]
    pub fn get_component(&self, req: GetComponentRequest) -> BridgeResult<GetComponentResponse> {
        self.validate_chain(req.chain_id)?;

        let composite_key = make_composite_key(req.chain_id, &req.component_id);

        if let Some(component) = self.state_cache.get_component(&composite_key) {
            let comp_info = ComponentInfo {
                id: format!("0x{}", hex::encode(&component.id)),
                protocol_system: component.protocol_system.clone(),
                protocol_type_name: component.protocol_type_name.clone(),
                chain_id: evm_chain_to_u64(&component.chain),
                tokens: component
                    .tokens
                    .iter()
                    .map(|t| format!("0x{}", hex::encode(&t.address)))
                    .collect(),
                tvl_usd: None,
                created_at: component.created_at.and_utc().timestamp(),
            };

            Ok(GetComponentResponse {
                component: Some(comp_info),
            })
        } else {
            Ok(GetComponentResponse { component: None })
        }
    }

    /// Get tokens asynchronously (required since TychoManager is async).
    #[instrument(skip(self))]
    pub async fn get_tokens_async(&self, req: GetTokensRequest) -> BridgeResult<GetTokensResponse> {
        if let Some(chain_id) = req.chain_id {
            self.validate_chain(chain_id)?;
        }

        let chain_tokens = self.tycho_manager.tokens().await;

        let mut matching_tokens: Vec<TokenInfo> = Vec::new();

        // Determine which chains to query
        let chains_to_query: Vec<u64> = if let Some(chain_id) = req.chain_id {
            if chain_id != 0 {
                vec![chain_id]
            } else {
                chain_tokens.keys().copied().collect()
            }
        } else {
            chain_tokens.keys().copied().collect()
        };

        for chain_id in chains_to_query {
            if let Some(tokens) = chain_tokens.get(&chain_id) {
                for (_addr, token) in tokens.iter() {
                    // Apply address filter if provided
                    if !req.addresses.is_empty() {
                        let token_addr =
                            format!("0x{}", hex::encode(&token.address)).to_lowercase();
                        if !req.addresses.iter().any(|a| a.to_lowercase() == token_addr) {
                            continue;
                        }
                    }

                    let token_info = TokenInfo {
                        address: format!("0x{}", hex::encode(&token.address)),
                        symbol: token.symbol.clone(),
                        decimals: token.decimals,
                        chain_id,
                    };
                    matching_tokens.push(token_info);
                }
            }
        }

        // Apply limit if provided
        if let Some(limit) = req.limit {
            matching_tokens.truncate(limit as usize);
        }

        info!(token_count = matching_tokens.len(), "GetTokens response");

        Ok(GetTokensResponse {
            tokens: matching_tokens,
        })
    }
}

/// Helper to calculate offset for limit-based pagination.
fn offset_for_limit(_limit: u32) -> usize {
    0 // For first page
}
