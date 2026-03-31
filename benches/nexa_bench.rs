//! Performance Benchmarks for Nexa-net
//!
//! Run with: cargo bench

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nexa_net::{
    discovery::{CapabilityRegistry, MockEmbedder, SemanticRouter, Vectorizer, VectorizerBuilder},
    economy::Channel,
    identity::{Did, KeyPair},
    storage::MemoryStore,
    types::{CapabilitySchema, ServiceMetadata},
};

// ============================================================================
// Identity Benchmarks
// ============================================================================

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("keypair_generation", |b| {
        b.iter(|| KeyPair::generate().unwrap())
    });
}

fn bench_did_creation(c: &mut Criterion) {
    c.bench_function("did_creation", |b| b.iter(|| Did::new("did:nexa:test123")));
}

fn bench_signing(c: &mut Criterion) {
    let keypair = KeyPair::generate().unwrap();
    let message = b"test message for signing benchmark";

    c.bench_function("sign_message", |b| {
        b.iter(|| keypair.sign(message).unwrap())
    });
}

fn bench_verification(c: &mut Criterion) {
    let keypair = KeyPair::generate().unwrap();
    let message = b"test message for signing benchmark";
    let signature = keypair.sign(message).unwrap();

    c.bench_function("verify_signature", |b| {
        b.iter(|| keypair.verify(message, &signature).unwrap())
    });
}

// ============================================================================
// Discovery Benchmarks
// ============================================================================

fn bench_vectorization(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let text = "translate English text to Chinese for document processing";

    c.bench_function("vectorize_text", |b| {
        b.iter(|| vectorizer.vectorize(text).unwrap())
    });
}

fn bench_batch_vectorization(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let texts: Vec<&str> = vec![
        "translate English to Chinese",
        "summarize this document",
        "generate code for sorting",
        "analyze sentiment of text",
        "extract entities from document",
    ];

    c.bench_function("vectorize_batch_5", |b| {
        b.iter(|| vectorizer.vectorize_batch(&texts).unwrap())
    });
}

fn bench_semantic_similarity(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let vec1 = vectorizer
        .vectorize("translate English to Chinese")
        .unwrap();
    let vec2 = vectorizer
        .vectorize("translation from English to Chinese")
        .unwrap();

    c.bench_function("cosine_similarity", |b| {
        b.iter(|| vec1.cosine_similarity(&vec2))
    });
}

fn bench_capability_registration(c: &mut Criterion) {
    let mut registry = CapabilityRegistry::new();

    c.bench_function("register_capability", |b| {
        b.iter_batched(
            || {
                let did = Did::new(&format!("did:nexa:bench{}", uuid::Uuid::new_v4()));
                CapabilitySchema {
                    version: "1.0".to_string(),
                    metadata: ServiceMetadata {
                        did,
                        name: "bench-service".to_string(),
                        description: "Benchmark service".to_string(),
                        tags: vec!["bench".to_string()],
                    },
                    endpoints: vec![],
                }
            },
            |schema| registry.register(schema).unwrap(),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ============================================================================
// Economy Benchmarks
// ============================================================================

fn bench_channel_creation(c: &mut Criterion) {
    let party_a = Did::new("did:nexa:party_a");
    let party_b = Did::new("did:nexa:party_b");

    c.bench_function("channel_creation", |b| {
        b.iter(|| Channel::new("channel-1", party_a.clone(), party_b.clone(), 1000, 500))
    });
}

fn bench_channel_transfer(c: &mut Criterion) {
    let party_a = Did::new("did:nexa:party_a");
    let party_b = Did::new("did:nexa:party_b");
    let mut channel = Channel::new("channel-1", party_a, party_b, 1000, 500);

    c.bench_function("channel_transfer", |b| {
        b.iter(|| {
            channel.transfer_a_to_b(10).unwrap();
            channel.transfer_b_to_a(10).unwrap();
        })
    });
}

// ============================================================================
// Storage Benchmarks
// ============================================================================

fn bench_storage_capability(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = MemoryStore::default_store();

    let schema = CapabilitySchema {
        version: "1.0".to_string(),
        metadata: ServiceMetadata {
            did: Did::new("did:nexa:bench"),
            name: "bench-service".to_string(),
            description: "Benchmark service".to_string(),
            tags: vec!["bench".to_string()],
        },
        endpoints: vec![],
    };

    c.bench_function("store_capability", |b| {
        b.to_async(&rt).iter(|| {
            let store = store.clone();
            async move { store.register_capability(schema.clone()).await.unwrap() }
        })
    });
}

fn bench_storage_cache(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = MemoryStore::default_store();
    let value = serde_json::json!({"key": "value", "data": [1, 2, 3, 4, 5]});

    c.bench_function("cache_set_get", |b| {
        b.to_async(&rt).iter(|| {
            let store = store.clone();
            let value = value.clone();
            async move {
                store.cache_set("bench-key", value.clone()).await.unwrap();
                store.cache_get("bench-key").await.unwrap()
            }
        })
    });
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn bench_vectorization_throughput(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let mut group = c.benchmark_group("vectorization_throughput");

    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), |b, &size| {
            let texts: Vec<String> = (0..size)
                .map(|i| format!("test intent number {} for benchmarking", i))
                .collect();
            let texts_ref: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

            b.iter(|| vectorizer.vectorize_batch(&texts_ref).unwrap())
        });
    }
    group.finish();
}

fn bench_signature_throughput(c: &mut Criterion) {
    let keypair = KeyPair::generate().unwrap();
    let mut group = c.benchmark_group("signature_throughput");

    for size in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), |b, &_size| {
            let message = b"test message for throughput benchmark";

            b.iter(|| {
                for _ in 0..100 {
                    keypair.sign(message).unwrap();
                }
            })
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    identity_benches,
    bench_keypair_generation,
    bench_did_creation,
    bench_signing,
    bench_verification,
);

criterion_group!(
    discovery_benches,
    bench_vectorization,
    bench_batch_vectorization,
    bench_semantic_similarity,
    bench_capability_registration,
);

criterion_group!(
    economy_benches,
    bench_channel_creation,
    bench_channel_transfer,
);

criterion_group!(
    storage_benches,
    bench_storage_capability,
    bench_storage_cache,
);

criterion_group!(
    throughput_benches,
    bench_vectorization_throughput,
    bench_signature_throughput,
);

criterion_main!(
    identity_benches,
    discovery_benches,
    economy_benches,
    storage_benches,
    throughput_benches,
);
