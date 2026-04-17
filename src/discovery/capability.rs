//! Capability Schema and Registry
//!
//! Implements the Capability Schema specification from DISCOVERY_LAYER.md.
//!
//! # Schema Structure
//!
//! ```yaml
//! nexa_capability:
//!   version: "1.0.0"
//!   metadata:
//!     did: "did:nexa:serviceprovider123..."
//!     name: "Advanced Translation Service"
//!     description: "Professional document translation"
//!     tags: ["translation", "document", "nlp"]
//!   endpoints:
//!     - id: "translate_document"
//!       name: "Document Translation"
//!       description: "Translate documents"
//!       input_schema: { ... }
//!       output_schema: { ... }
//!       cost:
//!         base: 10
//!         per_unit: 1
//! ```

use crate::error::{Error, Result};
use crate::types::{CapabilitySchema, EndpointDefinition};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Cost model for an endpoint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostModel {
    /// Base cost in NEXA tokens
    pub base: u64,
    /// Cost per unit (e.g., per 1KB, per second)
    pub per_unit: u64,
    /// Unit type (bytes, seconds, calls)
    pub unit_type: String,
    /// Free tier allowance
    pub free_tier: u64,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            base: 1,
            per_unit: 0,
            unit_type: "calls".to_string(),
            free_tier: 0,
        }
    }
}

/// Quality metrics for a service
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityMetrics {
    /// Average response time in milliseconds
    pub avg_response_time_ms: f32,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Uptime percentage (0.0 - 1.0)
    pub uptime: f32,
    /// Total calls served
    pub total_calls: u64,
    /// Rating (0.0 - 5.0)
    pub rating: f32,
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 100.0,
            success_rate: 1.0,
            uptime: 1.0,
            total_calls: 0,
            rating: 5.0,
        }
    }
}

/// Rate limit configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimit {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum requests per hour
    pub requests_per_hour: u32,
    /// Burst size
    pub burst_size: u32,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            requests_per_minute: 1000,
            requests_per_hour: 10000,
            burst_size: 50,
        }
    }
}

/// Extended endpoint definition with additional metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtendedEndpoint {
    /// Basic endpoint definition
    #[serde(flatten)]
    pub base: EndpointDefinition,
    /// Cost model
    pub cost: CostModel,
    /// Quality metrics
    pub quality: QualityMetrics,
    /// Rate limits
    pub rate_limit: RateLimit,
    /// Semantic vector (cached)
    #[serde(skip)]
    pub vector: Option<Vec<f32>>,
}

/// Registered capability with metadata
#[derive(Debug, Clone)]
pub struct RegisteredCapability {
    /// The capability schema
    pub schema: CapabilitySchema,
    /// Registration timestamp
    pub registered_at: Instant,
    /// Last update timestamp
    pub updated_at: Instant,
    /// Quality metrics
    pub quality: QualityMetrics,
    /// Whether the service is currently available
    pub available: bool,
    /// Semantic vector for the overall capability
    pub overall_vector: Option<Vec<f32>>,
    /// Semantic vectors per endpoint
    pub endpoint_vectors: HashMap<String, Vec<f32>>,
}

impl RegisteredCapability {
    /// Create a new registered capability
    pub fn new(schema: CapabilitySchema) -> Self {
        Self {
            schema,
            registered_at: Instant::now(),
            updated_at: Instant::now(),
            quality: QualityMetrics::default(),
            available: true,
            overall_vector: None,
            endpoint_vectors: HashMap::new(),
        }
    }

    /// Update the capability
    pub fn update(&mut self, schema: CapabilitySchema) {
        self.schema = schema;
        self.updated_at = Instant::now();
    }

    /// Set availability
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
        self.updated_at = Instant::now();
    }

    /// Update quality metrics
    pub fn update_quality(&mut self, quality: QualityMetrics) {
        self.quality = quality;
        self.updated_at = Instant::now();
    }

    /// Set semantic vector
    pub fn set_vector(&mut self, vector: Vec<f32>) {
        self.overall_vector = Some(vector);
    }

    /// Set endpoint vector
    pub fn set_endpoint_vector(&mut self, endpoint_id: String, vector: Vec<f32>) {
        self.endpoint_vectors.insert(endpoint_id, vector);
    }

    /// Get age of registration
    pub fn age(&self) -> Duration {
        self.registered_at.elapsed()
    }

    /// Get time since last update
    pub fn stale_time(&self) -> Duration {
        self.updated_at.elapsed()
    }
}

/// Capability registry for storing and querying capabilities
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    /// Registered capabilities by DID
    capabilities: HashMap<String, RegisteredCapability>,
    /// Tag index for fast lookup
    tag_index: HashMap<String, Vec<String>>,
    /// Maximum capabilities to store
    max_capabilities: usize,
    /// Stale timeout (capabilities older than this are considered stale)
    stale_timeout: Duration,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            tag_index: HashMap::new(),
            max_capabilities: 1000,
            stale_timeout: Duration::from_secs(3600),
        }
    }

    /// Create a registry with custom settings
    pub fn with_settings(max_capabilities: usize, stale_timeout: Duration) -> Self {
        Self {
            capabilities: HashMap::new(),
            tag_index: HashMap::new(),
            max_capabilities,
            stale_timeout,
        }
    }

    /// Register a capability
    pub fn register(&mut self, schema: CapabilitySchema) -> Result<()> {
        let did = schema.metadata.did.as_str().to_string();

        // Check if we're at capacity
        if !self.capabilities.contains_key(&did) && self.capabilities.len() >= self.max_capabilities
        {
            // Remove oldest capability
            if let Some((oldest_did, _)) = self
                .capabilities
                .iter()
                .min_by_key(|(_, cap)| cap.registered_at)
            {
                let oldest_did = oldest_did.clone();
                self.unregister(&oldest_did);
            }
        }

        // Build tag index
        for tag in &schema.metadata.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(did.clone());
        }

        // Create registered capability
        let registered = RegisteredCapability::new(schema);
        self.capabilities.insert(did, registered);

        Ok(())
    }

    /// Unregister a capability
    pub fn unregister(&mut self, did: &str) {
        if let Some(cap) = self.capabilities.remove(did) {
            // Remove from tag index
            for tag in &cap.schema.metadata.tags {
                if let Some(dids) = self.tag_index.get_mut(tag) {
                    dids.retain(|d| d != did);
                    if dids.is_empty() {
                        self.tag_index.remove(tag);
                    }
                }
            }
        }
    }

    /// Get a capability by DID
    pub fn get(&self, did: &str) -> Option<&CapabilitySchema> {
        self.capabilities.get(did).map(|r| &r.schema)
    }

    /// Get registered capability with metadata
    pub fn get_registered(&self, did: &str) -> Option<&RegisteredCapability> {
        self.capabilities.get(did)
    }

    /// Get mutable registered capability
    pub fn get_registered_mut(&mut self, did: &str) -> Option<&mut RegisteredCapability> {
        self.capabilities.get_mut(did)
    }

    /// List all capabilities
    pub fn list_all(&self) -> Vec<&CapabilitySchema> {
        self.capabilities.values().map(|r| &r.schema).collect()
    }

    /// List all registered capabilities
    pub fn list_all_registered(&self) -> Vec<&RegisteredCapability> {
        self.capabilities.values().collect()
    }

    /// Find capabilities by tags
    pub fn find_by_tags(&self, tags: &[String]) -> Vec<&CapabilitySchema> {
        let mut dids: Vec<&String> = Vec::new();

        for tag in tags {
            if let Some(tag_dids) = self.tag_index.get(tag) {
                for did in tag_dids {
                    if !dids.contains(&did) {
                        dids.push(did);
                    }
                }
            }
        }

        dids.into_iter()
            .filter_map(|did| self.capabilities.get(did).map(|r| &r.schema))
            .collect()
    }

    /// Find available capabilities
    pub fn find_available(&self) -> Vec<&CapabilitySchema> {
        self.capabilities
            .values()
            .filter(|r| r.available)
            .map(|r| &r.schema)
            .collect()
    }

    /// Find capabilities with quality above threshold
    pub fn find_by_quality(&self, min_success_rate: f32) -> Vec<&CapabilitySchema> {
        self.capabilities
            .values()
            .filter(|r| r.quality.success_rate >= min_success_rate)
            .map(|r| &r.schema)
            .collect()
    }

    /// Update capability availability
    pub fn set_availability(&mut self, did: &str, available: bool) -> Result<()> {
        let cap = self
            .capabilities
            .get_mut(did)
            .ok_or_else(|| Error::ServiceNotFound(did.to_string()))?;
        cap.set_available(available);
        Ok(())
    }

    /// Update capability quality metrics
    pub fn update_quality(&mut self, did: &str, quality: QualityMetrics) -> Result<()> {
        let cap = self
            .capabilities
            .get_mut(did)
            .ok_or_else(|| Error::ServiceNotFound(did.to_string()))?;
        cap.update_quality(quality);
        Ok(())
    }

    /// Set semantic vector for a capability
    pub fn set_vector(&mut self, did: &str, vector: Vec<f32>) -> Result<()> {
        let cap = self
            .capabilities
            .get_mut(did)
            .ok_or_else(|| Error::ServiceNotFound(did.to_string()))?;
        cap.set_vector(vector);
        Ok(())
    }

    /// Set endpoint vector for a capability
    pub fn set_endpoint_vector(
        &mut self,
        did: &str,
        endpoint_id: &str,
        vector: Vec<f32>,
    ) -> Result<()> {
        let cap = self
            .capabilities
            .get_mut(did)
            .ok_or_else(|| Error::ServiceNotFound(did.to_string()))?;
        cap.set_endpoint_vector(endpoint_id.to_string(), vector);
        Ok(())
    }

    /// Clean up stale capabilities
    pub fn cleanup_stale(&mut self) -> Vec<String> {
        let stale_dids: Vec<String> = self
            .capabilities
            .iter()
            .filter(|(_, cap)| cap.stale_time() > self.stale_timeout)
            .map(|(did, _)| did.clone())
            .collect();

        for did in &stale_dids {
            self.unregister(did);
        }

        stale_dids
    }

    /// Get registry statistics
    pub fn stats(&self) -> RegistryStats {
        let total = self.capabilities.len();
        let available = self.capabilities.values().filter(|c| c.available).count();
        let avg_quality = self
            .capabilities
            .values()
            .map(|c| c.quality.success_rate)
            .sum::<f32>()
            / total.max(1) as f32;

        RegistryStats {
            total_capabilities: total,
            available_capabilities: available,
            unique_tags: self.tag_index.len(),
            average_quality: avg_quality,
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Total capabilities registered
    pub total_capabilities: usize,
    /// Available capabilities
    pub available_capabilities: usize,
    /// Unique tags
    pub unique_tags: usize,
    /// Average quality (success rate)
    pub average_quality: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Did, ServiceMetadata};

    fn create_test_schema(did: &str, name: &str, tags: Vec<&str>) -> CapabilitySchema {
        CapabilitySchema {
            version: "1.0.0".to_string(),
            metadata: ServiceMetadata {
                did: Did::new(did),
                name: name.to_string(),
                description: format!("{} service", name),
                tags: tags.iter().map(|s| s.to_string()).collect(),
            },
            endpoints: vec![],
        }
    }

    #[test]
    fn test_registry() {
        let mut registry = CapabilityRegistry::new();
        let schema = create_test_schema("did:nexa:test", "Test Service", vec!["test"]);

        registry.register(schema).unwrap();
        assert!(registry.get("did:nexa:test").is_some());
    }

    #[test]
    fn test_find_by_tags() {
        let mut registry = CapabilityRegistry::new();

        registry
            .register(create_test_schema(
                "did:nexa:svc1",
                "Service 1",
                vec!["translation", "nlp"],
            ))
            .unwrap();
        registry
            .register(create_test_schema(
                "did:nexa:svc2",
                "Service 2",
                vec!["translation", "document"],
            ))
            .unwrap();
        registry
            .register(create_test_schema(
                "did:nexa:svc3",
                "Service 3",
                vec!["image", "vision"],
            ))
            .unwrap();

        let results = registry.find_by_tags(&["translation".to_string()]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_availability() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(create_test_schema("did:nexa:test", "Test", vec!["test"]))
            .unwrap();

        registry.set_availability("did:nexa:test", false).unwrap();

        let available = registry.find_available();
        assert!(available.is_empty());
    }

    #[test]
    fn test_quality_filtering() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(create_test_schema("did:nexa:test", "Test", vec!["test"]))
            .unwrap();

        registry
            .update_quality(
                "did:nexa:test",
                QualityMetrics {
                    success_rate: 0.8,
                    ..Default::default()
                },
            )
            .unwrap();

        let high_quality = registry.find_by_quality(0.9);
        assert!(high_quality.is_empty());

        let low_quality = registry.find_by_quality(0.7);
        assert_eq!(low_quality.len(), 1);
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(create_test_schema(
                "did:nexa:svc1",
                "Service 1",
                vec!["a", "b"],
            ))
            .unwrap();
        registry
            .register(create_test_schema(
                "did:nexa:svc2",
                "Service 2",
                vec!["b", "c"],
            ))
            .unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_capabilities, 2);
        assert_eq!(stats.unique_tags, 3);
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_empty_registry_search() {
        let registry = CapabilityRegistry::new();

        assert!(registry.get("did:nexa:any").is_none());
        assert!(registry
            .find_by_tags(&["translation".to_string()])
            .is_empty());
        assert!(registry.find_available().is_empty());
        assert!(registry.find_by_quality(0.0).is_empty());
        assert!(registry.list_all().is_empty());

        let stats = registry.stats();
        assert_eq!(stats.total_capabilities, 0);
        assert_eq!(stats.available_capabilities, 0);
        assert_eq!(stats.unique_tags, 0);
    }

    #[test]
    fn test_set_availability_nonexistent_did() {
        let mut registry = CapabilityRegistry::new();
        let result = registry.set_availability("did:nexa:nonexistent", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_quality_nonexistent_did() {
        let mut registry = CapabilityRegistry::new();
        let result = registry.update_quality("did:nexa:nonexistent", QualityMetrics::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_set_vector_nonexistent_did() {
        let mut registry = CapabilityRegistry::new();
        let result = registry.set_vector("did:nexa:nonexistent", vec![1.0, 2.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let mut registry = CapabilityRegistry::new();
        // unregister returns silently for nonexistent
        registry.unregister("did:nexa:nonexistent");
        assert_eq!(registry.list_all().len(), 0);
    }

    #[test]
    fn test_max_capabilities_eviction() {
        let mut registry = CapabilityRegistry::with_settings(3, Duration::from_secs(3600));

        for i in 0..5 {
            registry
                .register(create_test_schema(
                    &format!("did:nexa:svc{}", i),
                    &format!("Service {}", i),
                    vec!["test"],
                ))
                .unwrap();
        }

        // Should have at most 3 capabilities (evicted oldest)
        assert_eq!(registry.list_all().len(), 3);
    }

    #[test]
    fn test_find_by_multiple_tags_union() {
        let mut registry = CapabilityRegistry::new();

        registry
            .register(create_test_schema("did:nexa:svc1", "Svc1", vec!["a"]))
            .unwrap();
        registry
            .register(create_test_schema("did:nexa:svc2", "Svc2", vec!["b"]))
            .unwrap();
        registry
            .register(create_test_schema("did:nexa:svc3", "Svc3", vec!["a", "b"]))
            .unwrap();

        // Union search: searching for "a" OR "b" should return all three
        let results = registry.find_by_tags(&["a".to_string(), "b".to_string()]);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_find_by_nonexistent_tag() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(create_test_schema("did:nexa:svc1", "Svc1", vec!["a"]))
            .unwrap();

        let results = registry.find_by_tags(&["nonexistent".to_string()]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_unregister_cleans_tag_index() {
        let mut registry = CapabilityRegistry::new();

        registry
            .register(create_test_schema(
                "did:nexa:svc1",
                "Svc1",
                vec!["unique_tag"],
            ))
            .unwrap();
        assert_eq!(registry.stats().unique_tags, 1);

        registry.unregister("did:nexa:svc1");
        assert_eq!(registry.stats().unique_tags, 0);
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Register→query→tag search consistency with arbitrary tags
        #[test]
        fn proptest_capability_registry_tag_consistency(
            tags in prop::collection::vec("[a-z]{1,8}", 1..5),
        ) {
            let mut registry = CapabilityRegistry::new();

            let schema = CapabilitySchema {
                version: "1.0.0".to_string(),
                metadata: ServiceMetadata {
                    did: Did::new("did:nexa:test"),
                    name: "Test Service".to_string(),
                    description: "Test".to_string(),
                    tags: tags.clone(),
                },
                endpoints: vec![],
            };

            registry.register(schema).unwrap();

            // Every tag in the original set should find the capability
            for tag in &tags {
                let results = registry.find_by_tags(&[tag.clone()]);
                assert!(results.len() >= 1, "tag '{}' should find at least 1 result", tag);
            }
        }

        /// Multiple capabilities with overlapping tags — union search returns all
        #[test]
        fn proptest_registry_overlapping_tags(
            tag_sets in prop::collection::vec(prop::collection::vec("[a-z]{1,4}", 1..3), 1..5),
        ) {
            let mut registry = CapabilityRegistry::new();

            for (i, tags) in tag_sets.iter().enumerate() {
                let schema = CapabilitySchema {
                    version: "1.0.0".to_string(),
                    metadata: ServiceMetadata {
                        did: Did::new(format!("did:nexa:svc{}", i)),
                        name: format!("Service {}", i),
                        description: "Test".to_string(),
                        tags: tags.clone(),
                    },
                    endpoints: vec![],
                };
                registry.register(schema).unwrap();
            }

            // Stats should reflect all capabilities
            let stats = registry.stats();
            assert_eq!(stats.total_capabilities, tag_sets.len());
        }
    }
}
