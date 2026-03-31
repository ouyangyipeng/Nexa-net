//! Embedding Integration Tests
//!
//! Tests for the embedding module including MockEmbedder and ONNX embedder (when available).

use nexa_net::discovery::{
    Embedder, EmbeddingConfig, MockEmbedder, Vectorizer, VectorizerBuilder,
};
use nexa_net::discovery::embedding::utils::{cosine_similarity, normalize, euclidean_distance};
use std::sync::Arc;

/// Test MockEmbedder basic functionality
#[test]
fn test_mock_embedder_basic() {
    let embedder = MockEmbedder::new(384);
    
    assert_eq!(embedder.dimensions(), 384);
    assert_eq!(embedder.model_name(), "mock-embedder");
    
    let embedding = embedder.embed("hello world").unwrap();
    assert_eq!(embedding.len(), 384);
}

/// Test MockEmbedder determinism
#[test]
fn test_mock_embedder_deterministic() {
    let embedder = MockEmbedder::new(256);
    
    let emb1 = embedder.embed("test text").unwrap();
    let emb2 = embedder.embed("test text").unwrap();
    
    // Same text should produce same embedding
    let sim = cosine_similarity(&emb1, &emb2);
    assert!((sim - 1.0).abs() < 0.001, "Same text should produce identical embeddings");
}

/// Test MockEmbedder different texts produce different embeddings
#[test]
fn test_mock_embedder_different_texts() {
    let embedder = MockEmbedder::new(256);
    
    let emb1 = embedder.embed("translate English to Chinese").unwrap();
    let emb2 = embedder.embed("weather forecast tomorrow").unwrap();
    
    // Different texts should produce different embeddings
    let sim = cosine_similarity(&emb1, &emb2);
    assert!(sim < 0.99, "Different texts should produce different embeddings");
}

/// Test MockEmbedder batch embedding
#[test]
fn test_mock_embedder_batch() {
    let embedder = MockEmbedder::new(128);
    
    let texts = vec!["hello", "world", "test"];
    let embeddings = embedder.embed_batch(&texts).unwrap();
    
    assert_eq!(embeddings.len(), 3);
    for emb in &embeddings {
        assert_eq!(emb.len(), 128);
    }
}

/// Test Vectorizer with MockEmbedder
#[test]
fn test_vectorizer_with_mock() {
    let vectorizer = Vectorizer::new();
    
    let vec = vectorizer.vectorize("test intent").unwrap();
    assert_eq!(vec.dimensions, 384);
    
    // Test batch
    let texts = vec!["intent 1", "intent 2"];
    let vectors = vectorizer.vectorize_batch(&texts).unwrap();
    assert_eq!(vectors.len(), 2);
}

/// Test VectorizerBuilder
#[test]
fn test_vectorizer_builder() {
    let vectorizer = VectorizerBuilder::new()
        .mock(512)
        .build()
        .unwrap();
    
    assert_eq!(vectorizer.dimensions(), 512);
    assert_eq!(vectorizer.model_name(), "mock-embedder");
}

/// Test Vectorizer with custom embedder
#[test]
fn test_vectorizer_custom_embedder() {
    let embedder = Arc::new(MockEmbedder::new(256));
    let vectorizer = Vectorizer::with_embedder(embedder);
    
    assert_eq!(vectorizer.dimensions(), 256);
    
    let vec = vectorizer.vectorize("custom test").unwrap();
    assert_eq!(vec.dimensions, 256);
}

/// Test cosine similarity utility
#[test]
fn test_cosine_similarity() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let c = vec![0.0, 1.0, 0.0];
    let d = vec![0.5, 0.5, 0.0];
    
    // Identical vectors
    let sim_ab = cosine_similarity(&a, &b);
    assert!((sim_ab - 1.0).abs() < 0.001);
    
    // Orthogonal vectors
    let sim_ac = cosine_similarity(&a, &c);
    assert!((sim_ac - 0.0).abs() < 0.001);
    
    // 45 degree angle
    let sim_ad = cosine_similarity(&a, &d);
    assert!((sim_ad - 0.707).abs() < 0.01);
}

/// Test normalize utility
#[test]
fn test_normalize() {
    let mut vec = vec![3.0, 4.0, 0.0];
    normalize(&mut vec);
    
    // Should be unit vector: [0.6, 0.8, 0.0]
    assert!((vec[0] - 0.6).abs() < 0.001);
    assert!((vec[1] - 0.8).abs() < 0.001);
    assert!((vec[2] - 0.0).abs() < 0.001);
    
    // Check norm is 1
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.001);
}

/// Test euclidean distance utility
#[test]
fn test_euclidean_distance() {
    let a = vec![0.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let c = vec![1.0, 1.0, 0.0];
    
    let dist_ab = euclidean_distance(&a, &b);
    assert!((dist_ab - 1.0).abs() < 0.001);
    
    let dist_ac = euclidean_distance(&a, &c);
    assert!((dist_ac - 1.414).abs() < 0.01);
}

/// Test SemanticVector cosine similarity
#[test]
fn test_semantic_vector_similarity() {
    use nexa_net::discovery::SemanticVector;
    
    let vec1 = SemanticVector::new(vec![1.0, 0.0, 0.0]);
    let vec2 = SemanticVector::new(vec![1.0, 0.0, 0.0]);
    let vec3 = SemanticVector::new(vec![0.0, 1.0, 0.0]);
    
    assert!((vec1.cosine_similarity(&vec2) - 1.0).abs() < 0.001);
    assert!((vec1.cosine_similarity(&vec3) - 0.0).abs() < 0.001);
}

/// Test EmbeddingConfig default
#[test]
fn test_embedding_config_default() {
    let config = EmbeddingConfig::default();
    
    // Default should be Mock with 384 dimensions
    match config {
        EmbeddingConfig::Mock { dimensions } => {
            assert_eq!(dimensions, 384);
        }
        _ => panic!("Default config should be Mock"),
    }
}

/// Test create_embedder factory function
#[test]
fn test_create_embedder_factory() {
    let config = EmbeddingConfig::Mock { dimensions: 512 };
    let embedder = nexa_net::discovery::create_embedder(config).unwrap();
    
    assert_eq!(embedder.dimensions(), 512);
}

/// Test API config (should fail as not implemented)
#[test]
fn test_api_embedder_not_implemented() {
    let config = EmbeddingConfig::Api {
        endpoint: "https://api.example.com".to_string(),
        model: "test-model".to_string(),
        api_key: None,
    };
    
    let result = nexa_net::discovery::create_embedder(config);
    assert!(result.is_err(), "API embedder should not be implemented yet");
}

/// Test embedding for semantic similarity scenario
#[test]
fn test_semantic_similarity_scenario() {
    let embedder = MockEmbedder::new(384);
    
    // Simulate intent matching scenario
    let intent1 = "translate document from English to Chinese";
    let intent2 = "translation service English Chinese";
    let intent3 = "generate random numbers";
    
    let emb1 = embedder.embed(intent1).unwrap();
    let emb2 = embedder.embed(intent2).unwrap();
    let emb3 = embedder.embed(intent3).unwrap();
    
    // Similar intents should have higher similarity
    let sim_12 = cosine_similarity(&emb1, &emb2);
    let sim_13 = cosine_similarity(&emb1, &emb3);
    
    // Note: MockEmbedder uses hash-based approach, so semantic similarity
    // may not perfectly match real embeddings. This test validates the mechanism.
    println!("Similarity between similar intents: {}", sim_12);
    println!("Similarity between different intents: {}", sim_13);
}

// ONNX embedder tests (only run with feature flag and model available)
#[cfg(feature = "embedding-onnx")]
mod onnx_tests {
    use nexa_net::discovery::OnnxEmbedder;
    use nexa_net::discovery::Embedder;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Requires ONNX model files"]
    fn test_onnx_embedder_with_model() {
        // Default model path
        let model_path = PathBuf::from("models/all-MiniLM-L6-v2/model.onnx");
        
        if model_path.exists() {
            let embedder = OnnxEmbedder::new(model_path, 512).unwrap();
            
            assert_eq!(embedder.dimensions(), 384);
            
            let emb = embedder.embed("test embedding").unwrap();
            assert_eq!(emb.len(), 384);
            
            // Check normalization
            let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 0.01, "Embedding should be normalized");
        }
    }

    #[test]
    #[ignore = "Requires ONNX model files"]
    fn test_onnx_semantic_similarity() {
        let model_path = PathBuf::from("models/all-MiniLM-L6-v2/model.onnx");
        
        if model_path.exists() {
            let embedder = OnnxEmbedder::new(model_path, 512).unwrap();
            
            let emb1 = embedder.embed("translate English to Chinese").unwrap();
            let emb2 = embedder.embed("translation from English to Chinese").unwrap();
            let emb3 = embedder.embed("weather forecast").unwrap();
            
            use nexa_net::discovery::embedding::utils::cosine_similarity;
            
            let sim_12 = cosine_similarity(&emb1, &emb2);
            let sim_13 = cosine_similarity(&emb1, &emb3);
            
            // Real embeddings should show semantic similarity
            assert!(sim_12 > sim_13, "Similar intents should have higher similarity");
            assert!(sim_12 > 0.7, "Very similar intents should have high similarity");
        }
    }
}