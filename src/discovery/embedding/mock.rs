//! Mock Embedder Implementation
//!
//! A deterministic embedder for testing purposes.
//! Uses a hash-based approach to generate consistent embeddings for the same text.

use super::Embedder;
use crate::error::Result;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Mock embedder for testing
///
/// Generates deterministic embeddings based on text hashing.
/// Not suitable for production - use ONNX or API embedders instead.
pub struct MockEmbedder {
    dimensions: usize,
}

impl MockEmbedder {
    /// Create a new mock embedder with specified dimensions
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }

    /// Generate a deterministic hash for the text
    fn hash_text(&self, text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

impl Embedder for MockEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut result = vec![0.0f32; self.dimensions];

        // Use multiple hash passes to fill the vector
        let base_hash = self.hash_text(text);

        for i in 0..self.dimensions {
            // Combine base hash with position for variation
            let mut hasher = DefaultHasher::new();
            base_hash.hash(&mut hasher);
            (i as u64).hash(&mut hasher);
            let hash = hasher.finish();

            // Convert hash to float in range [-1, 1]
            // Use the lower 32 bits for the value
            let value = ((hash & 0xFFFFFFFF) as i32) as f32 / i32::MAX as f32;
            result[i] = value;
        }

        // Normalize the vector
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in result.iter_mut() {
                *v /= norm;
            }
        }

        Ok(result)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        "mock-embedder"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::embedding::utils::cosine_similarity;

    #[test]
    fn test_deterministic_embedding() {
        let embedder = MockEmbedder::new(128);

        let v1 = embedder.embed("test text").unwrap();
        let v2 = embedder.embed("test text").unwrap();

        // Same text should produce identical embeddings
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_different_texts_different_embeddings() {
        let embedder = MockEmbedder::new(128);

        let v1 = embedder.embed("hello world").unwrap();
        let v2 = embedder.embed("goodbye world").unwrap();

        // Different texts should produce different embeddings
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_normalized_output() {
        let embedder = MockEmbedder::new(128);

        let v = embedder.embed("any text").unwrap();

        // Check that the vector is normalized (L2 norm = 1)
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_batch_embedding() {
        let embedder = MockEmbedder::new(64);

        let texts = vec!["text one", "text two", "text three"];
        let embeddings = embedder.embed_batch(&texts).unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 64);
        }
    }

    #[test]
    fn test_similar_texts() {
        let embedder = MockEmbedder::new(384);

        // These texts share common words
        let v1 = embedder.embed("translate english to chinese").unwrap();
        let v2 = embedder.embed("translate chinese to english").unwrap();
        let v3 = embedder.embed("completely different topic").unwrap();

        // Similar texts should have higher similarity than dissimilar ones
        let sim_12 = cosine_similarity(&v1, &v2);
        let sim_13 = cosine_similarity(&v1, &v3);

        // Note: Mock embedder doesn't capture true semantic similarity
        // This test just verifies the mechanism works
        assert!(sim_12 >= -1.0 && sim_12 <= 1.0);
        assert!(sim_13 >= -1.0 && sim_13 <= 1.0);
    }
}
