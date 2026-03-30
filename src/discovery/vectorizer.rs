//! Semantic Vectorizer
//!
//! Converts text (intents, capabilities) into semantic vectors.

use crate::error::Result;

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
        
        let dot: f32 = self.data.iter()
            .zip(&other.data)
            .map(|(a, b)| a * b)
            .sum();
        
        let norm_a: f32 = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot / (norm_a * norm_b)
    }
}

/// Vectorizer for converting text to vectors
pub struct Vectorizer {
    /// Vector dimensions
    dimensions: usize,
}

impl Vectorizer {
    /// Create a new vectorizer
    pub fn new() -> Self {
        Self { dimensions: 384 }
    }
    
    /// Vectorize text
    /// 
    /// Note: In production, this would use an embedding model.
    /// For now, we use a simple hash-based approach for testing.
    pub fn vectorize(&self, text: &str) -> Result<SemanticVector> {
        // Simple hash-based vectorization for testing
        // TODO: Replace with actual embedding model
        let mut data = vec![0.0f32; self.dimensions];
        
        for (i, byte) in text.as_bytes().iter().enumerate() {
            let idx = (i + *byte as usize) % self.dimensions;
            data[idx] += (*byte as f32 / 255.0) - 0.5;
        }
        
        // Normalize
        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut data {
                *val /= norm;
            }
        }
        
        Ok(SemanticVector::new(data))
    }
}

impl Default for Vectorizer {
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
        let vec = vectorizer.vectorize("translate English to Chinese").unwrap();
        
        assert_eq!(vec.dimensions, 384);
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = SemanticVector::new(vec![1.0, 0.0, 0.0]);
        let vec2 = SemanticVector::new(vec![1.0, 0.0, 0.0]);
        let vec3 = SemanticVector::new(vec![0.0, 1.0, 0.0]);
        
        assert!((vec1.cosine_similarity(&vec2) - 1.0).abs() < 0.001);
        assert!((vec1.cosine_similarity(&vec3) - 0.0).abs() < 0.001);
    }
}