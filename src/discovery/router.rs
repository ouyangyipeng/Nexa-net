//! Semantic Router
//!
//! Routes requests to the most suitable service providers using multi-factor
//! semantic routing as specified in DISCOVERY_LAYER.md.
//!
//! # Routing Factors
//!
//! 1. **Semantic Similarity** - Cosine similarity between intent and capability vectors
//! 2. **Quality Score** - Success rate, response time, uptime
//! 3. **Cost Efficiency** - Cost per operation
//! 4. **Load Balance** - Current provider load
//! 5. **Latency** - Estimated response time

use crate::error::{Error, Result};
use crate::types::{Route, RouteContext};
use crate::discovery::{CapabilityRegistry, Vectorizer, NodeStatusManager};
use std::sync::Arc;
use std::time::Duration;

/// Routing weights for multi-factor scoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingWeights {
    /// Weight for semantic similarity (0.0 - 1.0)
    pub similarity: f32,
    /// Weight for quality score (0.0 - 1.0)
    pub quality: f32,
    /// Weight for cost efficiency (0.0 - 1.0)
    pub cost: f32,
    /// Weight for load balance (0.0 - 1.0)
    pub load: f32,
    /// Weight for latency (0.0 - 1.0)
    pub latency: f32,
}

impl Default for RoutingWeights {
    fn default() -> Self {
        Self {
            similarity: 0.4,
            quality: 0.25,
            cost: 0.15,
            load: 0.1,
            latency: 0.1,
        }
    }
}

impl RoutingWeights {
    /// Validate weights sum to 1.0
    pub fn validate(&self) -> bool {
        let sum = self.similarity + self.quality + self.cost + self.load + self.latency;
        (sum - 1.0).abs() < 0.01
    }

    /// Normalize weights to sum to 1.0
    pub fn normalize(&mut self) {
        let sum = self.similarity + self.quality + self.cost + self.load + self.latency;
        if sum > 0.0 {
            self.similarity /= sum;
            self.quality /= sum;
            self.cost /= sum;
            self.load /= sum;
            self.latency /= sum;
        }
    }
}

/// Routing configuration
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Routing weights
    pub weights: RoutingWeights,
    /// Minimum similarity threshold
    pub min_similarity: f32,
    /// Maximum cost threshold (0 = no limit)
    pub max_cost: u64,
    /// Maximum latency threshold in ms (0 = no limit)
    pub max_latency_ms: u32,
    /// Prefer available services only
    pub available_only: bool,
    /// Minimum quality threshold
    pub min_quality: f32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            weights: RoutingWeights::default(),
            min_similarity: 0.5,
            max_cost: 0,
            max_latency_ms: 0,
            available_only: true,
            min_quality: 0.8,
        }
    }
}

/// Routing candidate with computed scores
#[derive(Debug, Clone)]
pub struct RoutingCandidate {
    /// Provider DID
    pub provider_did: String,
    /// Endpoint ID
    pub endpoint_id: String,
    /// Endpoint name
    pub endpoint_name: String,
    /// Semantic similarity score (0.0 - 1.0)
    pub similarity_score: f32,
    /// Quality score (0.0 - 1.0)
    pub quality_score: f32,
    /// Cost score (0.0 - 1.0, higher = cheaper)
    pub cost_score: f32,
    /// Load score (0.0 - 1.0, higher = less loaded)
    pub load_score: f32,
    /// Latency score (0.0 - 1.0, higher = faster)
    pub latency_score: f32,
    /// Combined weighted score
    pub combined_score: f32,
    /// Estimated cost in NEXA
    pub estimated_cost: u64,
    /// Estimated latency in ms
    pub estimated_latency_ms: u32,
}

impl RoutingCandidate {
    /// Calculate combined score using weights
    pub fn calculate_combined_score(&mut self, weights: &RoutingWeights) {
        self.combined_score = 
            self.similarity_score * weights.similarity +
            self.quality_score * weights.quality +
            self.cost_score * weights.cost +
            self.load_score * weights.load +
            self.latency_score * weights.latency;
    }
}

/// Semantic router for service discovery
pub struct SemanticRouter {
    /// Capability registry
    registry: Arc<CapabilityRegistry>,
    /// Vectorizer
    vectorizer: Vectorizer,
    /// Node status manager
    node_status: Arc<NodeStatusManager>,
    /// Routing configuration
    config: RoutingConfig,
}

impl SemanticRouter {
    /// Create a new router
    pub fn new(registry: CapabilityRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
            vectorizer: Vectorizer::new(),
            node_status: Arc::new(NodeStatusManager::new()),
            config: RoutingConfig::default(),
        }
    }

    /// Create a router with shared references
    pub fn with_shared(
        registry: Arc<CapabilityRegistry>,
        node_status: Arc<NodeStatusManager>,
    ) -> Self {
        Self {
            registry,
            vectorizer: Vectorizer::new(),
            node_status,
            config: RoutingConfig::default(),
        }
    }

    /// Set routing configuration
    pub fn with_config(mut self, config: RoutingConfig) -> Self {
        self.config = config;
        self
    }

    /// Set routing weights
    pub fn set_weights(&mut self, weights: RoutingWeights) {
        self.config.weights = weights;
    }

    /// Get current configuration
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }

    /// Discover services matching an intent
    pub async fn discover(&self, intent: &str, context: RouteContext) -> Result<Vec<Route>> {
        let intent_vec = self.vectorizer.vectorize(intent)?;

        // Get all capabilities
        let capabilities = self.registry.list_all_registered();

        let mut candidates: Vec<RoutingCandidate> = Vec::new();

        for cap in capabilities {
            // Skip unavailable if configured
            if self.config.available_only && !cap.available {
                continue;
            }

            // Skip low quality if configured
            if cap.quality.success_rate < self.config.min_quality {
                continue;
            }

            // Calculate similarity
            let cap_text = format!("{} {}", 
                cap.schema.metadata.name, 
                cap.schema.metadata.description
            );
            let cap_vec = self.vectorizer.vectorize(&cap_text)?;
            let similarity = intent_vec.cosine_similarity(&cap_vec);

            // Skip low similarity
            if similarity < self.config.min_similarity {
                continue;
            }

            // Get node status for load info
            let node_status = self.node_status.get(&cap.schema.metadata.did.as_str().to_string());
            let load_score = node_status
                .map(|s| 1.0 - s.load)
                .unwrap_or(0.5);

            // Process each endpoint
            for endpoint in &cap.schema.endpoints {
                // Check cost threshold
                if self.config.max_cost > 0 && endpoint.base_cost > self.config.max_cost {
                    continue;
                }

                // Calculate scores
                let quality_score = cap.quality.success_rate;
                let cost_score = self.calculate_cost_score(endpoint.base_cost);
                let latency_score = self.calculate_latency_score(100); // TODO: actual latency

                let mut candidate = RoutingCandidate {
                    provider_did: cap.schema.metadata.did.as_str().to_string(),
                    endpoint_id: endpoint.id.clone(),
                    endpoint_name: endpoint.name.clone(),
                    similarity_score: similarity,
                    quality_score,
                    cost_score,
                    load_score,
                    latency_score,
                    combined_score: 0.0,
                    estimated_cost: endpoint.base_cost,
                    estimated_latency_ms: 100,
                };

                candidate.calculate_combined_score(&self.config.weights);
                candidates.push(candidate);
            }
        }

        // Sort by combined score
        candidates.sort_by(|a, b| {
            b.combined_score.partial_cmp(&a.combined_score).unwrap()
        });

        // Convert to routes and limit
        let routes: Vec<Route> = candidates
            .into_iter()
            .take(context.max_candidates)
            .map(|c| Route {
                endpoint: crate::types::EndpointDefinition {
                    id: c.endpoint_id,
                    name: c.endpoint_name,
                    description: String::new(),
                    input_schema: serde_json::Value::Null,
                    output_schema: serde_json::Value::Null,
                    base_cost: c.estimated_cost,
                    rate_limit: 100, // Default rate limit
                },
                provider_did: crate::types::Did::new(&c.provider_did),
                similarity_score: c.similarity_score,
                estimated_latency_ms: c.estimated_latency_ms as u64,
                estimated_cost: c.estimated_cost,
            })
            .collect();

        Ok(routes)
    }

    /// Select the best route
    pub async fn select_best(&self, intent: &str, context: RouteContext) -> Result<Route> {
        let routes = self.discover(intent, context).await?;

        routes.into_iter().next()
            .ok_or_else(|| Error::ServiceNotFound(intent.to_string()))
    }

    /// Discover with detailed candidates
    pub async fn discover_detailed(&self, intent: &str) -> Result<Vec<RoutingCandidate>> {
        let intent_vec = self.vectorizer.vectorize(intent)?;
        let capabilities = self.registry.list_all_registered();

        let mut candidates: Vec<RoutingCandidate> = Vec::new();

        for cap in capabilities {
            if self.config.available_only && !cap.available {
                continue;
            }

            let cap_text = format!("{} {}", 
                cap.schema.metadata.name, 
                cap.schema.metadata.description
            );
            let cap_vec = self.vectorizer.vectorize(&cap_text)?;
            let similarity = intent_vec.cosine_similarity(&cap_vec);

            if similarity < self.config.min_similarity {
                continue;
            }

            let node_status = self.node_status.get(&cap.schema.metadata.did.as_str().to_string());
            let load_score = node_status.map(|s| 1.0 - s.load).unwrap_or(0.5);

            for endpoint in &cap.schema.endpoints {
                let mut candidate = RoutingCandidate {
                    provider_did: cap.schema.metadata.did.as_str().to_string(),
                    endpoint_id: endpoint.id.clone(),
                    endpoint_name: endpoint.name.clone(),
                    similarity_score: similarity,
                    quality_score: cap.quality.success_rate,
                    cost_score: self.calculate_cost_score(endpoint.base_cost),
                    load_score,
                    latency_score: self.calculate_latency_score(100),
                    combined_score: 0.0,
                    estimated_cost: endpoint.base_cost,
                    estimated_latency_ms: 100,
                };

                candidate.calculate_combined_score(&self.config.weights);
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|a, b| {
            b.combined_score.partial_cmp(&a.combined_score).unwrap()
        });

        Ok(candidates)
    }

    /// Calculate cost score (higher = cheaper)
    fn calculate_cost_score(&self, cost: u64) -> f32 {
        // Exponential decay: score = e^(-cost/100)
        // Cost of 0 = score of 1.0
        // Cost of 100 = score of ~0.37
        // Cost of 1000 = score of ~0.0
        let cost_f32 = cost as f32;
        (-cost_f32 / 100.0).exp()
    }

    /// Calculate latency score (higher = faster)
    fn calculate_latency_score(&self, latency_ms: u32) -> f32 {
        // Exponential decay: score = e^(-latency/1000)
        // Latency of 0ms = score of 1.0
        // Latency of 100ms = score of ~0.9
        // Latency of 1000ms = score of ~0.37
        let latency_f32 = latency_ms as f32;
        (-latency_f32 / 1000.0).exp()
    }

    /// Update node status
    pub fn update_node_status(&self, did: &str, load: f32, latency_ms: u64) {
        let mut status = self.node_status.get(did)
            .cloned()
            .unwrap_or_else(|| crate::discovery::node_status::NodeStatus::new(did));
        
        status.load = load;
        status.avg_latency_ms = latency_ms;
        
        // Note: This would need interior mutability in production
        // For now, this is a placeholder
    }
}

/// Routing decision explanation
#[derive(Debug, Clone)]
pub struct RoutingExplanation {
    /// Selected provider DID
    pub provider_did: String,
    /// Selected endpoint ID
    pub endpoint_id: String,
    /// Why this route was selected
    pub reason: String,
    /// Individual factor scores
    pub factors: HashMap<String, f32>,
    /// Rejected alternatives
    pub rejected: Vec<(String, String)>,
}

use std::collections::HashMap;

impl SemanticRouter {
    /// Explain routing decision
    pub async fn explain(&self, intent: &str) -> Result<RoutingExplanation> {
        let candidates = self.discover_detailed(intent).await?;

        let best = candidates.first()
            .ok_or_else(|| Error::ServiceNotFound(intent.to_string()))?;

        let mut factors = HashMap::new();
        factors.insert("similarity".to_string(), best.similarity_score);
        factors.insert("quality".to_string(), best.quality_score);
        factors.insert("cost".to_string(), best.cost_score);
        factors.insert("load".to_string(), best.load_score);
        factors.insert("latency".to_string(), best.latency_score);

        let rejected: Vec<(String, String)> = candidates.iter()
            .skip(1)
            .take(5)
            .map(|c| {
                let reason = format!(
                    "Score: {:.3} (sim: {:.2}, qual: {:.2}, cost: {:.2})",
                    c.combined_score, c.similarity_score, c.quality_score, c.cost_score
                );
                (c.endpoint_name.clone(), reason)
            })
            .collect();

        Ok(RoutingExplanation {
            provider_did: best.provider_did.clone(),
            endpoint_id: best.endpoint_id.clone(),
            reason: format!(
                "Best combined score: {:.3} (similarity: {:.2}, quality: {:.2}, cost: {:.2})",
                best.combined_score, best.similarity_score, best.quality_score, best.cost_score
            ),
            factors,
            rejected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);
        assert_eq!(router.config().min_similarity, 0.5);
    }

    #[test]
    fn test_routing_weights() {
        let weights = RoutingWeights::default();
        assert!(weights.validate());
    }

    #[test]
    fn test_cost_score() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);

        assert!(router.calculate_cost_score(0) > router.calculate_cost_score(100));
        assert!(router.calculate_cost_score(100) > router.calculate_cost_score(1000));
    }

    #[test]
    fn test_latency_score() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);

        assert!(router.calculate_latency_score(0) > router.calculate_latency_score(100));
        assert!(router.calculate_latency_score(100) > router.calculate_latency_score(1000));
    }

    #[test]
    fn test_candidate_scoring() {
        let mut candidate = RoutingCandidate {
            provider_did: "did:nexa:test".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_name: "Test".to_string(),
            similarity_score: 0.9,
            quality_score: 0.95,
            cost_score: 0.8,
            load_score: 0.7,
            latency_score: 0.85,
            combined_score: 0.0,
            estimated_cost: 10,
            estimated_latency_ms: 100,
        };

        let weights = RoutingWeights::default();
        candidate.calculate_combined_score(&weights);

        assert!(candidate.combined_score > 0.0);
        assert!(candidate.combined_score <= 1.0);
    }
}