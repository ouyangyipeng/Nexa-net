//! Semantic Vectorizer
//!
//! Converts text (intents, capabilities) into semantic vectors.
//! Supports multiple embedding backends through the Embedder trait.

use crate::error::Result;
use std::sync::Arc;

// Re-export embedding types for convenience
pub use crate::discovery::embedding::mock::MockEmbedder;
pub use crate::discovery::embedding::{Embedder, EmbeddingConfig};

#[cfg(feature = "embedding-onnx")]
pub use crate::discovery::embedding::OnnxEmbedder;

/// Semantic vector representation
#[derive(Debug, Clone)]
pub struct SemanticVector {
    /// Vector dimensions
    pub dimensions: usize,
    /// Vector data
    pub data: Vec<f32>,
}

impl SemanticVector {
    /// Create a new vector
    pub fn new(data: Vec<f32>) -> Self {
        let dimensions = data.len();
        Self { dimensions, data }
    }

    /// Calculate cosine similarity with another vector
    pub fn cosine_similarity(&self, other: &SemanticVector) -> f32 {
        if self.dimensions != other.dimensions {
            return 0.0;
        }

        let dot: f32 = self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum();

        let norm_a: f32 = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// Create from raw embedding output
    pub fn from_embedding(data: Vec<f32>) -> Self {
        Self::new(data)
    }
}

/// Vectorizer for converting text to vectors
///
/// Uses an Embedder backend for actual text embedding.
/// Default backend is MockEmbedder for testing without external dependencies.
pub struct Vectorizer {
    /// Embedder backend
    embedder: Arc<dyn Embedder>,
}

impl Vectorizer {
    /// Create a new vectorizer with default MockEmbedder
    pub fn new() -> Self {
        Self {
            embedder: Arc::new(MockEmbedder::new(384)),
        }
    }

    /// Create a vectorizer with a specific embedder
    pub fn with_embedder(embedder: Arc<dyn Embedder>) -> Self {
        Self { embedder }
    }

    /// Create a vectorizer from configuration
    pub fn from_config(config: EmbeddingConfig) -> Result<Self> {
        let embedder = crate::discovery::embedding::create_embedder(config)?;
        Ok(Self { embedder })
    }

    /// Vectorize a single text
    pub fn vectorize(&self, text: &str) -> Result<SemanticVector> {
        let data = self.embedder.embed(text)?;
        Ok(SemanticVector::from_embedding(data))
    }

    /// Vectorize multiple texts in batch (more efficient)
    pub fn vectorize_batch(&self, texts: &[&str]) -> Result<Vec<SemanticVector>> {
        let embeddings = self.embedder.embed_batch(texts)?;
        Ok(embeddings
            .into_iter()
            .map(SemanticVector::from_embedding)
            .collect())
    }

    /// Get the dimensionality of vectors
    pub fn dimensions(&self) -> usize {
        self.embedder.dimensions()
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        self.embedder.model_name()
    }
}

impl Default for Vectorizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring Vectorizer
pub struct VectorizerBuilder {
    config: EmbeddingConfig,
}

impl VectorizerBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: EmbeddingConfig::default(),
        }
    }

    /// Use ONNX Runtime for local inference
    #[cfg(feature = "embedding-onnx")]
    pub fn onnx(mut self, model_path: std::path::PathBuf, max_length: usize) -> Self {
        self.config = EmbeddingConfig::Onnx {
            model_path,
            max_length,
        };
        self
    }

    /// Use HTTP API for remote inference
    pub fn api(mut self, endpoint: String, model: String, api_key: Option<String>) -> Self {
        self.config = EmbeddingConfig::Api {
            endpoint,
            model,
            api_key,
        };
        self
    }

    /// Use Mock embedder for testing
    pub fn mock(mut self, dimensions: usize) -> Self {
        self.config = EmbeddingConfig::Mock { dimensions };
        self
    }

    /// Build the Vectorizer
    pub fn build(self) -> Result<Vectorizer> {
        Vectorizer::from_config(self.config)
    }
}

impl Default for VectorizerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vectorize() {
        let vectorizer = Vectorizer::new();
        let vec = vectorizer
            .vectorize("translate English to Chinese")
            .unwrap();

        assert_eq!(vec.dimensions, 384);
        assert_eq!(vectorizer.dimensions(), 384);
        assert_eq!(vectorizer.model_name(), "mock-embedder");
    }

    #[test]
    fn test_vectorize_batch() {
        let vectorizer = Vectorizer::new();
        let texts = vec![
            "translate English to Chinese",
            "summarize this document",
            "generate code for sorting",
        ];
        let vectors = vectorizer.vectorize_batch(&texts).unwrap();

        assert_eq!(vectors.len(), 3);
        for vec in &vectors {
            assert_eq!(vec.dimensions, 384);
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = SemanticVector::new(vec![1.0, 0.0, 0.0]);
        let vec2 = SemanticVector::new(vec![1.0, 0.0, 0.0]);
        let vec3 = SemanticVector::new(vec![0.0, 1.0, 0.0]);

        assert!((vec1.cosine_similarity(&vec2) - 1.0).abs() < 0.001);
        assert!((vec1.cosine_similarity(&vec3) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_vectorizer_builder_mock() {
        let vectorizer = VectorizerBuilder::new().mock(256).build().unwrap();

        assert_eq!(vectorizer.dimensions(), 256);
    }

    #[test]
    fn test_vectorizer_builder_default() {
        let vectorizer = VectorizerBuilder::new().build().unwrap();

        // Default is Mock with 384 dimensions
        assert_eq!(vectorizer.dimensions(), 384);
    }

    #[test]
    fn test_from_embedding() {
        let data = vec![0.5, 0.3, 0.2, 0.1];
        let vec = SemanticVector::from_embedding(data.clone());

        assert_eq!(vec.dimensions, 4);
        assert_eq!(vec.data, data);
    }

    #[test]
    fn test_with_embedder() {
        let embedder = Arc::new(MockEmbedder::new(512));
        let vectorizer = Vectorizer::with_embedder(embedder);

        assert_eq!(vectorizer.dimensions(), 512);

        let vec = vectorizer.vectorize("test text").unwrap();
        assert_eq!(vec.dimensions, 512);
    }
}
