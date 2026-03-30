//! DID Resolver
//!
//! Resolves DID documents from the distributed storage.

use crate::error::{Error, Result};
use crate::identity::{Did, DidDocument};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// DID resolution result
#[derive(Debug, Clone)]
pub struct DidResolutionResult {
    /// The resolved DID document
    pub document: Option<DidDocument>,
    /// Resolution metadata
    pub metadata: ResolutionMetadata,
}

/// Resolution metadata
#[derive(Debug, Clone)]
pub struct ResolutionMetadata {
    /// Resolution time
    pub resolved_at: Instant,
    /// Source of the document
    pub source: ResolutionSource,
    /// Whether the result came from cache
    pub from_cache: bool,
    /// Error message if resolution failed
    pub error: Option<String>,
}

/// Source of DID document
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionSource {
    /// From local cache
    Cache,
    /// From DHT network
    DHT,
    /// From local storage
    Local,
    /// From supernode
    Supernode,
}

/// DID Resolver
#[derive(Debug, Clone)]
pub struct DidResolver {
    /// Local cache of DID documents
    cache: HashMap<String, CachedDocument>,
    /// Cache TTL
    cache_ttl: Duration,
}

/// Cached DID document
#[derive(Debug, Clone)]
struct CachedDocument {
    /// The cached document
    document: DidDocument,
    /// When it was cached
    cached_at: Instant,
}

impl DidResolver {
    /// Create a new DID resolver
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(3600), // 1 hour default TTL
        }
    }
    
    /// Set cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }
    
    /// Resolve a DID to its document
    pub async fn resolve(&mut self, did: &Did) -> Result<DidResolutionResult> {
        let did_str = did.as_str();
        
        // Check cache first
        if let Some(cached) = self.cache.get(did_str) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                return Ok(DidResolutionResult {
                    document: Some(cached.document.clone()),
                    metadata: ResolutionMetadata {
                        resolved_at: Instant::now(),
                        source: ResolutionSource::Cache,
                        from_cache: true,
                        error: None,
                    },
                });
            }
        }
        
        // TODO: Query DHT network for DID document
        // For now, return an error indicating the document was not found
        Ok(DidResolutionResult {
            document: None,
            metadata: ResolutionMetadata {
                resolved_at: Instant::now(),
                source: ResolutionSource::DHT,
                from_cache: false,
                error: Some("DID document not found in network".to_string()),
            },
        })
    }
    
    /// Resolve with verification
    pub async fn resolve_with_verification(&mut self, did: &Did) -> Result<DidResolutionResult> {
        let result = self.resolve(did).await?;
        
        if let Some(doc) = &result.document {
            // Verify the document's integrity
            if !self.verify_document(did, doc) {
                return Ok(DidResolutionResult {
                    document: None,
                    metadata: ResolutionMetadata {
                        resolved_at: Instant::now(),
                        source: result.metadata.source,
                        from_cache: result.metadata.from_cache,
                        error: Some("DID document verification failed".to_string()),
                    },
                });
            }
        }
        
        Ok(result)
    }
    
    /// Cache a DID document
    pub fn cache_document(&mut self, did: &Did, document: DidDocument) {
        self.cache.insert(did.as_str().to_string(), CachedDocument {
            document,
            cached_at: Instant::now(),
        });
    }
    
    /// Invalidate cache for a DID
    pub fn invalidate_cache(&mut self, did: &Did) {
        self.cache.remove(did.as_str());
    }
    
    /// Clear all cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Verify a DID document
    fn verify_document(&self, did: &Did, document: &DidDocument) -> bool {
        // Check that the document ID matches the DID
        if document.id != did.as_str() {
            return false;
        }
        
        // TODO: Verify signatures and proof
        true
    }
    
    /// Register a DID document locally
    pub fn register(&mut self, did: &Did, document: DidDocument) {
        self.cache_document(did, document);
    }
}

impl Default for DidResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::key_management::KeyPair;

    #[test]
    fn test_resolver_creation() {
        let resolver = DidResolver::new();
        assert!(resolver.cache.is_empty());
    }

    #[test]
    fn test_cache_document() {
        let mut resolver = DidResolver::new();
        let keypair = KeyPair::generate().unwrap();
        let did = Did::from_public_key(keypair.public_key().inner());
        let doc = DidDocument::new(&did, keypair.public_key().inner());
        
        resolver.cache_document(&did, doc.clone());
        
        assert!(resolver.cache.contains_key(did.as_str()));
    }

    #[test]
    fn test_invalidate_cache() {
        let mut resolver = DidResolver::new();
        let keypair = KeyPair::generate().unwrap();
        let did = Did::from_public_key(keypair.public_key().inner());
        let doc = DidDocument::new(&did, keypair.public_key().inner());
        
        resolver.cache_document(&did, doc);
        resolver.invalidate_cache(&did);
        
        assert!(!resolver.cache.contains_key(did.as_str()));
    }
}