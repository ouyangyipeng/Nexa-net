//! Nexa-net Embedding Usage Example
//!
//! This example demonstrates the embedding functionality:
//! - Creating embedders (Mock, ONNX, API)
//! - Vectorizing text
//! - Computing semantic similarity
//! - Using the vectorizer for discovery

use nexa_net::{
    discovery::embedding::{create_embedder, Embedder, EmbeddingConfig},
    discovery::semantic_dht::SemanticDHT,
    discovery::vectorizer::{Vectorizer, VectorizerBuilder},
    discovery::SemanticVector,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Nexa-net Embedding Example ===\n");

    // =========================================================================
    // 1. Mock Embedder (for testing)
    // =========================================================================
    println!("1. Mock Embedder (Testing)...");

    let mock_config = EmbeddingConfig::Mock { dimensions: 384 };
    let mock_embedder = create_embedder(mock_config)?;

    // Generate embeddings
    let text1 = "Translate this text to French";
    let text2 = "Convert this document to German";
    let text3 = "Generate a summary of this article";

    let vec1 = mock_embedder.embed(text1)?;
    let vec2 = mock_embedder.embed(text2)?;
    let vec3 = mock_embedder.embed(text3)?;

    println!("   Generated {}-dimensional vectors", vec1.len());
    println!("   Text 1: \"{}\"", text1);
    println!("   Text 2: \"{}\"", text2);
    println!("   Text 3: \"{}\"", text3);

    // =========================================================================
    // 2. Semantic Similarity
    // =========================================================================
    println!("\n2. Computing Semantic Similarity...");

    use nexa_net::discovery::embedding::utils::cosine_similarity;

    let sim_12 = cosine_similarity(&vec1, &vec2);
    let sim_13 = cosine_similarity(&vec1, &vec3);
    let sim_23 = cosine_similarity(&vec2, &vec3);

    println!("   Similarity(text1, text2): {:.4}", sim_12);
    println!("   Similarity(text1, text3): {:.4}", sim_13);
    println!("   Similarity(text2, text3): {:.4}", sim_23);

    // Note: Mock embedder produces deterministic but not semantically meaningful vectors
    // In production, use ONNX or API embedder for real semantic similarity

    // =========================================================================
    // 3. Vectorizer Usage
    // =========================================================================
    println!("\n3. Vectorizer Usage...");

    // Create a vectorizer with mock embedder
    let vectorizer = VectorizerBuilder::new().mock(384).build();

    // Vectorize multiple texts
    let texts = vec![
        "Machine translation service",
        "Document summarization API",
        "Image recognition endpoint",
    ];

    let vectors = vectorizer.vectorize_batch(&texts)?;
    println!("   Vectorized {} texts", vectors.len());

    // Compute pairwise similarities
    println!("   Pairwise similarities:");
    for i in 0..vectors.len() {
        for j in (i + 1)..vectors.len() {
            let sim = vectors[i].cosine_similarity(&vectors[j]);
            println!("     [{}] vs [{}]: {:.4}", i, j, sim);
        }
    }

    // =========================================================================
    // 4. Semantic DHT
    // =========================================================================
    println!("\n4. Semantic DHT (Distributed Hash Table)...");

    let mut dht = SemanticDHT::new();

    // Store capabilities with their embeddings
    let capabilities = vec![
        (
            "did:nexa:svc:translation",
            "Neural machine translation service",
        ),
        (
            "did:nexa:svc:summarization",
            "Document summarization and extraction",
        ),
        (
            "did:nexa:svc:vision",
            "Image classification and object detection",
        ),
        (
            "did:nexa:svc:speech",
            "Speech-to-text transcription service",
        ),
    ];

    for (did, description) in &capabilities {
        let vector = vectorizer.vectorize(description)?;
        dht.store(did.to_string(), vector);
    }

    println!("   Stored {} capabilities in DHT", capabilities.len());

    // Query for similar capabilities
    let query = "I need to convert audio to text";
    let query_vector = vectorizer.vectorize(query)?;

    let similar = dht.find_similar(&query_vector, 0.0); // threshold = 0.0 for mock

    println!("   Query: \"{}\"", query);
    println!("   Found {} similar capabilities:", similar.len());
    for (did, score) in similar {
        println!("     {} (score: {:.4})", did, score);
    }

    // =========================================================================
    // 5. Embedding Utilities
    // =========================================================================
    println!("\n5. Embedding Utilities...");

    use nexa_net::discovery::embedding::utils::{euclidean_distance, normalize};

    // Normalize a vector
    let mut vec = vec1.clone();
    normalize(&mut vec);
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    println!("   Normalized vector L2 norm: {:.6} (should be ~1.0)", norm);

    // Euclidean distance
    let dist = euclidean_distance(&vec1, &vec2);
    println!("   Euclidean distance(vec1, vec2): {:.4}", dist);

    // =========================================================================
    // 6. Configuration Options
    // =========================================================================
    println!("\n6. Configuration Options...");

    // ONNX configuration (requires model file)
    let onnx_config = EmbeddingConfig::Onnx {
        model_path: "./models/all-MiniLM-L6-v2.onnx".into(),
        tokenizer_path: "./models/tokenizer.json".into(),
        max_length: 256,
    };
    println!("   ONNX config: model at ./models/");

    // API configuration (requires API endpoint)
    let api_config = EmbeddingConfig::Api {
        endpoint: "https://api.example.com/embeddings".to_string(),
        model: "text-embedding-ada-002".to_string(),
        api_key: std::env::var("EMBEDDING_API_KEY").ok(),
    };
    println!("   API config: endpoint at https://api.example.com/embeddings");

    println!("\n=== Example Complete ===");
    Ok(())
}
