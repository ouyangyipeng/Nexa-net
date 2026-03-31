//! Embedding Module
//!
//! Provides text embedding capabilities for semantic similarity computation.
//! Supports multiple backends: ONNX Runtime (local), HTTP API (remote), and Mock (testing).

use crate::error::Result;
use std::sync::Arc;

#[cfg(feature = "embedding-onnx")]
pub mod onnx;

pub mod mock;

/// Embedder trait - abstract interface for text embedding
///
/// Implementations can use different backends:
/// - ONNX Runtime for local inference
/// - HTTP API for remote embedding services
/// - Mock for testing
pub trait Embedder: Send + Sync {
    /// Embed a single text into a vector
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed multiple texts in batch (more efficient for large volumes)
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Default implementation: sequential embedding
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Get the dimensionality of the embedding vectors
    fn dimensions(&self) -> usize;

    /// Get the name of the embedding model
    fn model_name(&self) -> &str;
}

/// Configuration for embedding backends
#[derive(Debug, Clone)]
pub enum EmbeddingConfig {
    /// ONNX Runtime local inference
    #[cfg(feature = "embedding-onnx")]
    Onnx {
        /// Path to the ONNX model file
        model_path: std::path::PathBuf,
        /// Maximum sequence length
        max_length: usize,
    },

    /// HTTP API remote inference
    Api {
        /// API endpoint URL
        endpoint: String,
        /// API key (if required)
        api_key: Option<String>,
        /// Model identifier
        model: String,
    },

    /// Mock embedder for testing
    Mock {
        /// Fixed dimensionality
        dimensions: usize,
    },
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self::Mock { dimensions: 384 }
    }
}

/// Factory function to create an embedder based on configuration
pub fn create_embedder(config: EmbeddingConfig) -> Result<Arc<dyn Embedder>> {
    match config {
        #[cfg(feature = "embedding-onnx")]
        EmbeddingConfig::Onnx { model_path, max_length } => {
            Ok(Arc::new(onnx::OnnxEmbedder::new(model_path, max_length)?))
        }

        EmbeddingConfig::Mock { dimensions } => {
            Ok(Arc::new(mock::MockEmbedder::new(dimensions)))
        }

        EmbeddingConfig::Api { .. } => {
            // API embedder not yet implemented
            Err(crate::error::Error::Config(
                "API embedder not yet implemented".to_string()
            ))
        }
    }
}

/// Utility functions for vector operations
pub mod utils {
    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// Normalize a vector in-place
    pub fn normalize(vector: &mut [f32]) {
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vector.iter_mut() {
                *v /= norm;
            }
        }
    }

    /// Compute Euclidean distance between two vectors
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::MAX;
        }

        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((utils::cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((utils::cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((utils::cosine_similarity(&a, &d) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        utils::normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_mock_embedder() {
        let embedder = mock::MockEmbedder::new(384);
        assert_eq!(embedder.dimensions(), 384);

        let v1 = embedder.embed("hello world").unwrap();
        assert_eq!(v1.len(), 384);

        // Same text should produce same embedding
        let v2 = embedder.embed("hello world").unwrap();
        assert!((utils::cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-6);
    }
}