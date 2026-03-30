//! Semantic DHT (Distributed Hash Table)
//!
//! Provides distributed storage and retrieval of semantic vectors.

use crate::error::Result;
use crate::discovery::vectorizer::SemanticVector;
use std::collections::HashMap;

/// DHT node information
#[derive(Debug, Clone)]
pub struct DhtNode {
    /// Node ID
    pub id: String,
    /// Node address
    pub address: String,
}

/// Semantic DHT for storing and querying vectors
#[derive(Debug, Clone, Default)]
pub struct SemanticDHT {
    /// Local storage
    storage: HashMap<String, SemanticVector>,
    /// Known nodes
    nodes: Vec<DhtNode>,
}

impl SemanticDHT {
    /// Create a new DHT
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Store a vector
    pub fn store(&mut self, key: String, vector: SemanticVector) -> Result<()> {
        self.storage.insert(key, vector);
        Ok(())
    }
    
    /// Retrieve a vector
    pub fn get(&self, key: &str) -> Option<&SemanticVector> {
        self.storage.get(key)
    }
    
    /// Find similar vectors
    pub fn find_similar(&self, query: &SemanticVector, threshold: f32) -> Vec<(String, f32)> {
        self.storage
            .iter()
            .map(|(key, vec)| {
                let similarity = query.cosine_similarity(vec);
                (key.clone(), similarity)
            })
            .filter(|(_, sim)| *sim >= threshold)
            .collect()
    }
    
    /// Add a node
    pub fn add_node(&mut self, node: DhtNode) {
        self.nodes.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_store_retrieve() {
        let mut dht = SemanticDHT::new();
        let vec = SemanticVector::new(vec![1.0, 0.0, 0.0]);
        
        dht.store("key1".to_string(), vec.clone()).unwrap();
        let retrieved = dht.get("key1");
        
        assert!(retrieved.is_some());
    }
}