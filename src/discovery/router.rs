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

use crate::discovery::{CapabilityRegistry, NodeStatusManager, Vectorizer};
use crate::error::{Error, Result};
use crate::types::{Route, RouteContext};
use std::sync::Arc;
use tokio::sync::RwLock;

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
        self.combined_score = self.similarity_score * weights.similarity
            + self.quality_score * weights.quality
            + self.cost_score * weights.cost
            + self.load_score * weights.load
            + self.latency_score * weights.latency;
    }
}

/// Semantic router for service discovery
pub struct SemanticRouter {
    /// Capability registry (shared with ProxyState via Arc<RwLock> for concurrent read/write)
    registry: Arc<RwLock<CapabilityRegistry>>,
    /// Vectorizer
    vectorizer: Vectorizer,
    /// Node status manager (shared with ProxyState via Arc<RwLock>)
    node_status: Arc<RwLock<NodeStatusManager>>,
    /// Routing configuration
    config: RoutingConfig,
}

impl SemanticRouter {
    /// Create a new router (wraps registry in Arc<RwLock> for standalone use)
    pub fn new(registry: CapabilityRegistry) -> Self {
        Self {
            registry: Arc::new(RwLock::new(registry)),
            vectorizer: Vectorizer::new(),
            node_status: Arc::new(RwLock::new(NodeStatusManager::new())),
            config: RoutingConfig::default(),
        }
    }

    /// Create a router with shared references (for ProxyState integration)
    ///
    /// Both registry and node_status are shared Arc<RwLock> instances,
    /// so writes via ProxyState are visible to this router's discover calls.
    pub fn with_shared(
        registry: Arc<RwLock<CapabilityRegistry>>,
        node_status: Arc<RwLock<NodeStatusManager>>,
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

        // Read registry under lock, clone data, then release lock before async work
        let capabilities: Vec<crate::discovery::capability::RegisteredCapability> = {
            let registry = self.registry.read().await;
            registry
                .list_all_registered()
                .into_iter()
                .cloned()
                .collect()
        };

        let mut candidates: Vec<RoutingCandidate> = Vec::new();

        for cap in &capabilities {
            // Skip unavailable if configured
            if self.config.available_only && !cap.available {
                continue;
            }

            // Skip low quality if configured
            if cap.quality.success_rate < self.config.min_quality {
                continue;
            }

            // Calculate similarity
            let cap_text = format!(
                "{} {}",
                cap.schema.metadata.name, cap.schema.metadata.description
            );
            let cap_vec = self.vectorizer.vectorize(&cap_text)?;
            let similarity = intent_vec.cosine_similarity(&cap_vec);

            // Skip low similarity
            if similarity < self.config.min_similarity {
                continue;
            }

            // Get node status for load info (read lock, brief access)
            let load_score = {
                let node_status = self.node_status.read().await;
                node_status
                    .get(cap.schema.metadata.did.as_str())
                    .map(|s| 1.0 - s.load)
                    .unwrap_or(0.5)
            };

            // Process each endpoint
            for endpoint in &cap.schema.endpoints {
                // Check cost threshold
                if self.config.max_cost > 0 && endpoint.base_cost > self.config.max_cost {
                    continue;
                }

                // Calculate scores
                let quality_score = cap.quality.success_rate;
                let cost_score = self.calculate_cost_score(endpoint.base_cost);
                let latency_score = self.calculate_latency_score(100); // NOTE: Placeholder latency — actual value from node monitoring

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
        candidates.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

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

        routes
            .into_iter()
            .next()
            .ok_or_else(|| Error::ServiceNotFound(intent.to_string()))
    }

    /// Discover with detailed candidates
    pub async fn discover_detailed(&self, intent: &str) -> Result<Vec<RoutingCandidate>> {
        let intent_vec = self.vectorizer.vectorize(intent)?;

        let capabilities: Vec<crate::discovery::capability::RegisteredCapability> = {
            let registry = self.registry.read().await;
            registry
                .list_all_registered()
                .into_iter()
                .cloned()
                .collect()
        };

        let mut candidates: Vec<RoutingCandidate> = Vec::new();

        for cap in &capabilities {
            if self.config.available_only && !cap.available {
                continue;
            }

            let cap_text = format!(
                "{} {}",
                cap.schema.metadata.name, cap.schema.metadata.description
            );
            let cap_vec = self.vectorizer.vectorize(&cap_text)?;
            let similarity = intent_vec.cosine_similarity(&cap_vec);

            if similarity < self.config.min_similarity {
                continue;
            }

            let load_score = {
                let node_status = self.node_status.read().await;
                node_status
                    .get(cap.schema.metadata.did.as_str())
                    .map(|s| 1.0 - s.load)
                    .unwrap_or(0.5)
            };

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

        candidates.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

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

    /// Update node status (async due to RwLock write guard)
    pub async fn update_node_status(&self, did: &str, load: f32, latency_ms: u64) {
        let mut node_status = self.node_status.write().await;
        let mut status = node_status
            .get(did)
            .cloned()
            .unwrap_or_else(|| crate::discovery::node_status::NodeStatus::new(did));

        status.load = load;
        status.avg_latency_ms = latency_ms;

        node_status.update(status);
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

        let best = candidates
            .first()
            .ok_or_else(|| Error::ServiceNotFound(intent.to_string()))?;

        let mut factors = HashMap::new();
        factors.insert("similarity".to_string(), best.similarity_score);
        factors.insert("quality".to_string(), best.quality_score);
        factors.insert("cost".to_string(), best.cost_score);
        factors.insert("load".to_string(), best.load_score);
        factors.insert("latency".to_string(), best.latency_score);

        let rejected: Vec<(String, String)> = candidates
            .iter()
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

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_routing_weights_validate_invalid() {
        let weights = RoutingWeights {
            similarity: 0.5,
            quality: 0.5,
            cost: 0.5,
            load: 0.5,
            latency: 0.5,
        };
        // Sum = 2.5, not 1.0
        assert!(!weights.validate());
    }

    #[test]
    fn test_routing_weights_validate_zero_sum() {
        let weights = RoutingWeights {
            similarity: 0.0,
            quality: 0.0,
            cost: 0.0,
            load: 0.0,
            latency: 0.0,
        };
        assert!(!weights.validate());
    }

    #[test]
    fn test_routing_weights_normalize() {
        let mut weights = RoutingWeights {
            similarity: 2.0,
            quality: 1.0,
            cost: 1.0,
            load: 0.5,
            latency: 0.5,
        };
        weights.normalize();

        // After normalization, sum should be ≈1.0
        let sum =
            weights.similarity + weights.quality + weights.cost + weights.load + weights.latency;
        assert!((sum - 1.0).abs() < 0.01);
        assert!(weights.validate());
    }

    #[test]
    fn test_routing_weights_normalize_zero_sum_no_effect() {
        let mut weights = RoutingWeights {
            similarity: 0.0,
            quality: 0.0,
            cost: 0.0,
            load: 0.0,
            latency: 0.0,
        };
        weights.normalize();
        // Zero sum should not cause division by zero; weights remain zero
        assert_eq!(weights.similarity, 0.0);
    }

    #[test]
    fn test_cost_score_monotonically_decreasing() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);

        let scores: Vec<f32> = [0, 10, 50, 100, 500, 1000]
            .iter()
            .map(|c| router.calculate_cost_score(*c))
            .collect();

        for i in 1..scores.len() {
            assert!(
                scores[i - 1] > scores[i],
                "cost_score({}) = {} should be > cost_score({}) = {}",
                [0, 10, 50, 100, 500, 1000][i - 1],
                scores[i - 1],
                [0, 10, 50, 100, 500, 1000][i],
                scores[i]
            );
        }
    }

    #[test]
    fn test_latency_score_monotonically_decreasing() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);

        let scores: Vec<f32> = [0, 50, 100, 500, 1000, 5000]
            .iter()
            .map(|l| router.calculate_latency_score(*l))
            .collect();

        for i in 1..scores.len() {
            assert!(
                scores[i - 1] > scores[i],
                "latency_score({}) = {} should be > latency_score({}) = {}",
                [0, 50, 100, 500, 1000, 5000][i - 1],
                scores[i - 1],
                [0, 50, 100, 500, 1000, 5000][i],
                scores[i]
            );
        }
    }

    #[test]
    fn test_cost_score_zero_is_one() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);
        // e^0 = 1.0
        let score = router.calculate_cost_score(0);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_latency_score_zero_is_one() {
        let registry = CapabilityRegistry::new();
        let router = SemanticRouter::new(registry);
        // e^0 = 1.0
        let score = router.calculate_latency_score(0);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_candidate_combined_score_zero_weights() {
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

        let weights = RoutingWeights {
            similarity: 0.0,
            quality: 0.0,
            cost: 0.0,
            load: 0.0,
            latency: 0.0,
        };
        candidate.calculate_combined_score(&weights);
        assert!((candidate.combined_score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_routing_config_default_values() {
        let config = RoutingConfig::default();
        assert_eq!(config.min_similarity, 0.5);
        assert_eq!(config.max_cost, 0);
        assert_eq!(config.max_latency_ms, 0);
        assert!(config.available_only);
        assert_eq!(config.min_quality, 0.8);
    }

    #[test]
    fn test_router_set_weights() {
        let registry = CapabilityRegistry::new();
        let mut router = SemanticRouter::new(registry);

        let weights = RoutingWeights {
            similarity: 0.6,
            quality: 0.2,
            cost: 0.1,
            load: 0.05,
            latency: 0.05,
        };
        router.set_weights(weights);

        let config = router.config();
        assert!((config.weights.similarity - 0.6).abs() < 0.01);
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Routing weights normalize → validate: after normalization, sum ≈ 1.0
        #[test]
        fn proptest_routing_weights_normalize(
            sim in 0.01f32..5.0,
            qual in 0.01f32..5.0,
            cost in 0.01f32..5.0,
            load in 0.01f32..5.0,
            lat in 0.01f32..5.0,
        ) {
            let mut weights = RoutingWeights {
                similarity: sim,
                quality: qual,
                cost: cost,
                load: load,
                latency: lat,
            };
            weights.normalize();

            let sum = weights.similarity + weights.quality + weights.cost + weights.load + weights.latency;
            assert!((sum - 1.0).abs() < 0.01, "normalized sum should be 1.0, got {}", sum);
            assert!(weights.validate());
        }

        /// Cost score is always in (0, 1] and decreases with cost
        #[test]
        fn proptest_cost_score_range(cost in 0u64..10000) {
            let registry = CapabilityRegistry::new();
            let router = SemanticRouter::new(registry);
            let score = router.calculate_cost_score(cost);
            assert!(score > 0.0 && score <= 1.0, "cost_score({}) = {} out of range (0,1]", cost, score);
        }

        /// Latency score is always in (0, 1] and decreases with latency
        #[test]
        fn proptest_latency_score_range(latency in 0u32..10000) {
            let registry = CapabilityRegistry::new();
            let router = SemanticRouter::new(registry);
            let score = router.calculate_latency_score(latency);
            assert!(score > 0.0 && score <= 1.0, "latency_score({}) = {} out of range (0,1]", latency, score);
        }
    }
}
