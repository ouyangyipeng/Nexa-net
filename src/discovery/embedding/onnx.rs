//! ONNX Runtime Embedder Implementation
//!
//! Uses ONNX Runtime for local inference of sentence embedding models.
//! Supports models like all-MiniLM-L6-v2, paraphrase-multilingual-MiniLM-L12-v2, etc.
//! Integrates proper tokenization via the `tokenizers` crate.

use super::Embedder;
use crate::error::{Error, Result};

#[cfg(feature = "embedding-onnx")]
use ort::{GraphOptimizationLevel, Session};

#[cfg(feature = "embedding-onnx")]
use tokenizers::Tokenizer;

/// ONNX Runtime based embedder
///
/// Loads a pre-trained ONNX model and performs local inference.
/// Models should be exported from HuggingFace transformers or similar.
pub struct OnnxEmbedder {
    #[cfg(feature = "embedding-onnx")]
    session: Session,
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
        use std::fs;

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

        // Load ONNX session
        let session = Session::builder()
            .map_err(|e| Error::Config(format!("Failed to create ONNX session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| Error::Config(format!("Failed to set optimization level: {}", e)))?
            .commit_from_file(&model_path)
            .map_err(|e| Error::Config(format!("Failed to load ONNX model: {}", e)))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::Config(format!("Failed to load tokenizer: {}", e)))?;

        // Get input/output info
        let input_count = session.inputs.len();
        let output_count = session.outputs.len();

        if input_count == 0 || output_count == 0 {
            return Err(Error::Config(
                "Invalid ONNX model: no inputs or outputs".to_string(),
            ));
        }

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
            session,
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
        use std::fs;

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

        // Load ONNX session
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
            session,
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
        use tokenizers::PaddingOptions;
        use tokenizers::TruncationOptions;

        // Configure truncation and padding
        let tokenizer = self
            .tokenizer
            .with_truncation(Some(TruncationOptions {
                max_length: self.max_length,
                stride: 0,
                strategy: tokenizers::TruncationStrategy::LongestFirst,
            }))
            .map_err(|_| "Failed to set truncation")
            .unwrap_or_else(|_| self.tokenizer.clone());

        // Encode the text
        let encoding = tokenizer
            .encode(text, true)
            .map_err(|_| "Failed to encode text")
            .unwrap_or_else(|_| tokenizers::Encoding::default());

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|id| *id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|m| *m as i64)
            .collect();

        (input_ids, attention_mask)
    }
}

#[cfg(feature = "embedding-onnx")]
impl Embedder for OnnxEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        use ndarray::{Array1, Array2, Axis};
        use ort::inputs;

        let (tokens, attention_mask) = self.tokenize(text);
        let token_count = tokens.len();

        if token_count == 0 {
            return Ok(vec![0.0; self.dimensions]);
        }

        // Create input arrays
        let input_ids = Array1::from_vec(tokens).insert_axis(Axis(0)).to_owned();
        let attention_mask_arr = Array1::from_vec(attention_mask)
            .insert_axis(Axis(0))
            .to_owned();

        // Run inference
        let outputs = self
            .session
            .run(
                inputs![
                    "input_ids" => input_ids.view(),
                    "attention_mask" => attention_mask_arr.view()
                ]
                .map_err(|e| Error::Internal(format!("Failed to create inputs: {}", e)))?,
            )
            .map_err(|e| Error::Internal(format!("ONNX inference failed: {}", e)))?;

        // Extract the embedding (mean pooling over sequence dimension)
        // Output shape: [1, seq_len, hidden_size]
        let output = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| Error::Internal(format!("Failed to extract output: {}", e)))?;

        let shape = output.shape();
        let hidden_size = shape[2];

        // Mean pooling with attention mask weighting
        let mut embedding = vec![0.0f32; hidden_size];
        let mut mask_sum = 0.0f32;

        for i in 0..token_count {
            let mask_val = attention_mask_arr[[0, i]] as f32;
            mask_sum += mask_val;
            for j in 0..hidden_size {
                embedding[j] += output[[0, i, j]] * mask_val;
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
        unreachable!("OnnxEmbedder::new should fail without feature")
    }

    fn dimensions(&self) -> usize {
        0
    }

    fn model_name(&self) -> &str {
        "unavailable"
    }
}

#[cfg(all(test, feature = "embedding-onnx"))]
mod tests {
    use super::*;

    // Note: These tests require actual ONNX model and tokenizer files
    // They are disabled by default and only run when models are available

    #[test]
    #[ignore = "Requires ONNX model and tokenizer files"]
    fn test_onnx_embedder_load() {
        let model_path = std::path::PathBuf::from("models/all-MiniLM-L6-v2/model.onnx");
        if model_path.exists() {
            let embedder = OnnxEmbedder::new(model_path, 512).unwrap();
            assert_eq!(embedder.dimensions(), 384);
            assert!(embedder.model_name().contains("MiniLM"));
        }
    }

    #[test]
    #[ignore = "Requires ONNX model and tokenizer files"]
    fn test_onnx_embedding() {
        let model_path = std::path::PathBuf::from("models/all-MiniLM-L6-v2/model.onnx");
        if model_path.exists() {
            let embedder = OnnxEmbedder::new(model_path, 512).unwrap();
            let embedding = embedder.embed("hello world").unwrap();
            assert_eq!(embedding.len(), 384);

            // Check normalization
            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 0.01);
        }
    }

    #[test]
    #[ignore = "Requires ONNX model and tokenizer files"]
    fn test_onnx_batch_embedding() {
        let model_path = std::path::PathBuf::from("models/all-MiniLM-L6-v2/model.onnx");
        if model_path.exists() {
            let embedder = OnnxEmbedder::new(model_path, 512).unwrap();
            let texts = vec!["hello world", "test embedding", "semantic search"];
            let embeddings = embedder.embed_batch(&texts).unwrap();
            assert_eq!(embeddings.len(), 3);
            for emb in &embeddings {
                assert_eq!(emb.len(), 384);
            }
        }
    }

    #[test]
    #[ignore = "Requires ONNX model and tokenizer files"]
    fn test_onnx_similarity() {
        let model_path = std::path::PathBuf::from("models/all-MiniLM-L6-v2/model.onnx");
        if model_path.exists() {
            let embedder = OnnxEmbedder::new(model_path, 512).unwrap();

            let emb1 = embedder.embed("translate English to Chinese").unwrap();
            let emb2 = embedder
                .embed("translation from English to Chinese")
                .unwrap();
            let emb3 = embedder.embed("weather forecast tomorrow").unwrap();

            // Similar texts should have higher similarity
            let sim12 = super::super::utils::cosine_similarity(&emb1, &emb2);
            let sim13 = super::super::utils::cosine_similarity(&emb1, &emb3);

            assert!(
                sim12 > sim13,
                "Similar texts should have higher cosine similarity"
            );
        }
    }
}
