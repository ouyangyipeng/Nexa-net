//! ONNX Runtime Embedder Implementation
//!
//! Uses ONNX Runtime for local inference of sentence embedding models.
//! Supports models like all-MiniLM-L6-v2, paraphrase-multilingual-MiniLM-L12-v2, etc.
//! Integrates proper tokenization via the `tokenizers` crate.

use super::Embedder;
use crate::error::{Error, Result};

#[cfg(feature = "embedding-onnx")]
use ort::session::Session;

#[cfg(feature = "embedding-onnx")]
use ort::session::builder::GraphOptimizationLevel;

#[cfg(feature = "embedding-onnx")]
use ort::value::{Shape, Value};

#[cfg(feature = "embedding-onnx")]
use tokenizers::Tokenizer;

#[cfg(feature = "embedding-onnx")]
use std::sync::Mutex;

/// ONNX Runtime based embedder
///
/// Loads a pre-trained ONNX model and performs local inference.
/// Models should be exported from HuggingFace transformers or similar.
pub struct OnnxEmbedder {
    #[cfg(feature = "embedding-onnx")]
    session: Mutex<Session>,
    #[cfg(feature = "embedding-onnx")]
    tokenizer: Tokenizer,
    dimensions: usize,
    max_length: usize,
    model_name: String,
}

impl OnnxEmbedder {
    /// Create a new ONNX embedder
    ///
    /// # Arguments
    /// * `model_path` - Path to the ONNX model file
    /// * `max_length` - Maximum sequence length for tokenization
    ///
    /// # Model Directory Structure
    /// Expected files in the model directory:
    /// - `model.onnx` or `<name>.onnx` - The ONNX model file
    /// - `tokenizer.json` - HuggingFace tokenizer configuration
    #[cfg(feature = "embedding-onnx")]
    pub fn new(model_path: std::path::PathBuf, max_length: usize) -> Result<Self> {
        if !model_path.exists() {
            return Err(Error::Config(format!(
                "ONNX model not found at: {:?}",
                model_path
            )));
        }

        // Look for tokenizer.json in the same directory
        let tokenizer_path = model_path
            .parent()
            .map(|p| p.join("tokenizer.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("tokenizer.json"));

        if !tokenizer_path.exists() {
            return Err(Error::Config(format!(
                "Tokenizer not found at: {:?}. Please provide tokenizer.json from HuggingFace.",
                tokenizer_path
            )));
        }

        // Load ONNX session with new API
        let session = Session::builder()
            .map_err(|e| Error::Config(format!("Failed to create ONNX session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| Error::Config(format!("Failed to set optimization level: {}", e)))?
            .commit_from_file(&model_path)
            .map_err(|e| Error::Config(format!("Failed to load ONNX model: {}", e)))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::Config(format!("Failed to load tokenizer: {}", e)))?;

        // Extract model name from path
        let model_name = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Default dimensions for common models
        // all-MiniLM-L6-v2: 384
        // paraphrase-multilingual-MiniLM-L12-v2: 384
        // all-mpnet-base-v2: 768
        let dimensions = Self::detect_dimensions(&model_name);

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            dimensions,
            max_length,
            model_name,
        })
    }

    /// Create a new ONNX embedder with explicit tokenizer path
    #[cfg(feature = "embedding-onnx")]
    pub fn with_tokenizer(
        model_path: std::path::PathBuf,
        tokenizer_path: std::path::PathBuf,
        max_length: usize,
    ) -> Result<Self> {
        if !model_path.exists() {
            return Err(Error::Config(format!(
                "ONNX model not found at: {:?}",
                model_path
            )));
        }

        if !tokenizer_path.exists() {
            return Err(Error::Config(format!(
                "Tokenizer not found at: {:?}",
                tokenizer_path
            )));
        }

        // Load ONNX session with new API
        let session = Session::builder()
            .map_err(|e| Error::Config(format!("Failed to create ONNX session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| Error::Config(format!("Failed to set optimization level: {}", e)))?
            .commit_from_file(&model_path)
            .map_err(|e| Error::Config(format!("Failed to load ONNX model: {}", e)))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::Config(format!("Failed to load tokenizer: {}", e)))?;

        // Extract model name from path
        let model_name = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let dimensions = Self::detect_dimensions(&model_name);

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            dimensions,
            max_length,
            model_name,
        })
    }

    /// Detect embedding dimensions from model name
    fn detect_dimensions(model_name: &str) -> usize {
        if model_name.contains("mpnet") || model_name.contains("base-v2") {
            768
        } else if model_name.contains("large") {
            1024
        } else {
            // Default for MiniLM models
            384
        }
    }

    /// Tokenize text using the loaded tokenizer
    #[cfg(feature = "embedding-onnx")]
    fn tokenize(&self, text: &str) -> (Vec<i64>, Vec<i64>) {
        use tokenizers::TruncationDirection;
        use tokenizers::TruncationStrategy;

        // Clone tokenizer and configure truncation
        let mut tokenizer = self.tokenizer.clone();

        // Set truncation using the new API
        let truncation_params = tokenizers::TruncationParams {
            max_length: self.max_length,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
            direction: TruncationDirection::Right,
        };

        if let Err(e) = tokenizer.with_truncation(Some(truncation_params)) {
            tracing::warn!("Failed to set truncation: {}", e);
        }

        // Encode the text
        match tokenizer.encode(text, true) {
            Ok(encoding) => {
                let input_ids: Vec<i64> = encoding.get_ids().iter().map(|id| *id as i64).collect();
                let attention_mask: Vec<i64> = encoding
                    .get_attention_mask()
                    .iter()
                    .map(|m| *m as i64)
                    .collect();
                (input_ids, attention_mask)
            }
            Err(e) => {
                tracing::warn!("Failed to encode text: {}", e);
                (vec![], vec![])
            }
        }
    }
}

#[cfg(feature = "embedding-onnx")]
impl Embedder for OnnxEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let (tokens, attention_mask) = self.tokenize(text);
        let token_count = tokens.len();

        if token_count == 0 {
            return Ok(vec![0.0; self.dimensions]);
        }

        // Create input values using (shape, data) tuple format for ort 2.0
        // Shape is [1, seq_len] for batch dimension
        let input_ids_shape = Shape::new(vec![1, token_count as i64]);
        let attention_mask_shape = Shape::new(vec![1, token_count as i64]);

        // Create ONNX input values using (shape, data) tuple
        let input_ids_value = Value::from_array((input_ids_shape, tokens.into_boxed_slice()))
            .map_err(|e| Error::Internal(format!("Failed to create input_ids value: {}", e)))?;

        let attention_mask_value = Value::from_array((
            attention_mask_shape,
            attention_mask.clone().into_boxed_slice(),
        ))
        .map_err(|e| Error::Internal(format!("Failed to create attention_mask value: {}", e)))?;

        // Run inference - need to lock the session for mutable access
        let mut session = self
            .session
            .lock()
            .map_err(|_| Error::Internal("Failed to lock ONNX session".to_string()))?;

        let outputs = session
            .run(vec![
                ("input_ids", input_ids_value),
                ("attention_mask", attention_mask_value),
            ])
            .map_err(|e| Error::Internal(format!("ONNX inference failed: {}", e)))?;

        // Extract the embedding output
        let output_value = &outputs[0];

        // Get the tensor data - ort 2.0 returns (&Shape, &[T])
        let (_shape, output_slice) = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| Error::Internal(format!("Failed to extract output: {}", e)))?;

        // Use the detected dimensions for the model
        let hidden_size = self.dimensions;

        // Mean pooling with attention mask weighting
        let mut embedding = vec![0.0f32; hidden_size];
        let mut mask_sum = 0.0f32;

        // Process each token
        #[allow(clippy::needless_range_loop)]
        for i in 0..token_count {
            let mask_val = attention_mask[i] as f32;
            mask_sum += mask_val;
            #[allow(clippy::needless_range_loop)]
            for j in 0..hidden_size {
                let idx = i * hidden_size + j;
                if idx < output_slice.len() {
                    embedding[j] += output_slice[idx] * mask_val;
                }
            }
        }

        // Normalize by mask sum
        if mask_sum > 0.0 {
            for val in embedding.iter_mut() {
                *val /= mask_sum;
            }
        }

        // L2 normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Process each text individually
        // For better performance, consider batching at the ONNX level
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Stub implementation when ONNX feature is disabled
#[cfg(not(feature = "embedding-onnx"))]
impl OnnxEmbedder {
    pub fn new(_model_path: std::path::PathBuf, _max_length: usize) -> Result<Self> {
        Err(Error::Config(
            "ONNX embedder requires 'embedding-onnx' feature".to_string(),
        ))
    }

    pub fn with_tokenizer(
        _model_path: std::path::PathBuf,
        _tokenizer_path: std::path::PathBuf,
        _max_length: usize,
    ) -> Result<Self> {
        Err(Error::Config(
            "ONNX embedder requires 'embedding-onnx' feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "embedding-onnx"))]
impl Embedder for OnnxEmbedder {
    fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        Err(Error::Config(
            "ONNX embedder requires 'embedding-onnx' feature".to_string(),
        ))
    }

    fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Err(Error::Config(
            "ONNX embedder requires 'embedding-onnx' feature".to_string(),
        ))
    }

    fn dimensions(&self) -> usize {
        384
    }

    fn model_name(&self) -> &str {
        "unavailable"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onnx_embedder_load() {
        // This test requires a model file, so we test the stub behavior
        let result = OnnxEmbedder::new(std::path::PathBuf::from("nonexistent.onnx"), 256);
        #[cfg(feature = "embedding-onnx")]
        assert!(result.is_err());
        #[cfg(not(feature = "embedding-onnx"))]
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_dimensions() {
        assert_eq!(OnnxEmbedder::detect_dimensions("all-mpnet-base-v2"), 768);
        assert_eq!(OnnxEmbedder::detect_dimensions("all-MiniLM-L6-v2"), 384);
        assert_eq!(OnnxEmbedder::detect_dimensions("some-large-model"), 1024);
    }
}
