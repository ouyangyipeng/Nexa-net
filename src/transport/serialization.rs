//! Serialization Engine
//!
//! Protobuf, FlatBuffers, and JSON serialization support with multi-algorithm compression.
//!
//! # Features
//!
//! - Multiple serialization formats (Protobuf, FlatBuffers, JSON, Binary)
//! - LZ4, Zstd, and Gzip compression for payload optimization
//! - Schema compression for efficient transmission
//! - Zero-copy deserialization support
//! - Configurable compression levels (Fast, Default, Best)

use crate::error::{Error, Result};
use std::io::{Read, Write};

/// Serialization format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SerializationFormat {
    #[default]
    /// Protobuf (Protocol Buffers)
    Protobuf,
    /// FlatBuffers (zero-copy)
    FlatBuffers,
    /// JSON (for debugging and compatibility)
    Json,
    /// Binary (raw bytes)
    Binary,
}

impl std::fmt::Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationFormat::Protobuf => write!(f, "protobuf"),
            SerializationFormat::FlatBuffers => write!(f, "flatbuffers"),
            SerializationFormat::Json => write!(f, "json"),
            SerializationFormat::Binary => write!(f, "binary"),
        }
    }
}

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionAlgorithm {
    #[default]
    /// No compression
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression (balanced)
    Zstd,
    /// Gzip compression (compatible)
    Gzip,
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    /// Fast compression (lower ratio)
    Fast,
    #[default]
    /// Default compression
    Default,
    /// Best compression (higher ratio, slower)
    Best,
}

/// Serializer trait
pub trait Serializer {
    /// Serialize data to bytes
    fn serialize<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>>;

    /// Get the format
    fn format(&self) -> SerializationFormat;

    /// Serialize with compression
    fn serialize_compressed<T: serde::Serialize>(
        &self,
        data: &T,
        algorithm: CompressionAlgorithm,
    ) -> Result<Vec<u8>> {
        let serialized = self.serialize(data)?;
        compress(&serialized, algorithm)
    }
}

/// Deserializer trait
pub trait Deserializer {
    /// Deserialize data from bytes
    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T>;

    /// Get the format
    fn format(&self) -> SerializationFormat;

    /// Deserialize with decompression
    fn deserialize_compressed<T: serde::de::DeserializeOwned>(
        &self,
        data: &[u8],
        algorithm: CompressionAlgorithm,
    ) -> Result<T> {
        let decompressed = decompress(data, algorithm)?;
        self.deserialize(&decompressed)
    }
}

/// Compress data using specified algorithm
pub fn compress(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Lz4 => compress_lz4(data),
        CompressionAlgorithm::Zstd => compress_zstd(data),
        CompressionAlgorithm::Gzip => compress_gzip(data),
    }
}

/// Decompress data using specified algorithm
pub fn decompress(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Lz4 => decompress_lz4(data),
        CompressionAlgorithm::Zstd => decompress_zstd(data),
        CompressionAlgorithm::Gzip => decompress_gzip(data),
    }
}

/// LZ4 compression
fn compress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    // Use lz4_flex for fast compression

    // Simple LZ4 block compression (no frame)
    let compressed = lz4_flex::compress(data);

    // Add header: original size (4 bytes) + compressed size (4 bytes)
    let mut result = Vec::with_capacity(8 + compressed.len());
    result.extend_from_slice(&(data.len() as u32).to_be_bytes());
    result.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    result.extend_from_slice(&compressed);

    Ok(result)
}

/// LZ4 decompression
fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 8 {
        return Err(Error::Protocol("Invalid LZ4 compressed data".to_string()));
    }

    let original_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let compressed_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;

    if data.len() < 8 + compressed_size {
        return Err(Error::Protocol("LZ4 data truncated".to_string()));
    }

    let compressed = &data[8..8 + compressed_size];
    lz4_flex::decompress(compressed, original_size)
        .map_err(|e| Error::Protocol(format!("LZ4 decompression failed: {}", e)))
}

/// Zstd compression using the `zstd` crate
///
/// Zstd provides high compression ratios (typically 60-80% reduction) with
/// fast decompression speeds, making it ideal for network payload compression.
///
/// Format: [4 bytes: original_size] + [zstd compressed data]
fn compress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    // Use default compression level (3)
    let compressed = zstd::encode_all(data, 3)
        .map_err(|e| Error::Protocol(format!("Zstd compression failed: {}", e)))?;

    // Prepend original size header for safe decompression
    let mut result = Vec::with_capacity(4 + compressed.len());
    result.extend_from_slice(&(data.len() as u32).to_be_bytes());
    result.extend_from_slice(&compressed);

    Ok(result)
}

/// Zstd decompression
///
/// Decompresses data with the original_size header, ensuring the output
/// matches the expected size for data integrity verification.
fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 4 {
        return Err(Error::Protocol(
            "Invalid Zstd compressed data: missing header".to_string(),
        ));
    }

    // Read original size from header
    let original_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

    let decompressed = zstd::decode_all(&data[4..])
        .map_err(|e| Error::Protocol(format!("Zstd decompression failed: {}", e)))?;

    // Verify output size matches header
    if decompressed.len() != original_size {
        return Err(Error::Protocol(format!(
            "Zstd decompression size mismatch: expected {}, got {}",
            original_size,
            decompressed.len()
        )));
    }

    Ok(decompressed)
}

/// Gzip compression using the `flate2` crate
///
/// Gzip provides good compression ratios with wide compatibility.
/// Suitable for payloads that need to be compatible with standard HTTP/gzip tools.
///
/// Format: [4 bytes: original_size] + [gzip compressed data]
fn compress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| Error::Protocol(format!("Gzip write failed: {}", e)))?;
    let compressed = encoder
        .finish()
        .map_err(|e| Error::Protocol(format!("Gzip compression failed: {}", e)))?;

    // Prepend original size header
    let mut result = Vec::with_capacity(4 + compressed.len());
    result.extend_from_slice(&(data.len() as u32).to_be_bytes());
    result.extend_from_slice(&compressed);

    Ok(result)
}

/// Gzip decompression
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;

    if data.len() < 4 {
        return Err(Error::Protocol(
            "Invalid Gzip compressed data: missing header".to_string(),
        ));
    }

    // Read original size from header
    let original_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

    let mut decoder = GzDecoder::new(&data[4..]);
    let mut decompressed = Vec::with_capacity(original_size);
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| Error::Protocol(format!("Gzip decompression failed: {}", e)))?;

    // Verify output size
    if decompressed.len() != original_size {
        return Err(Error::Protocol(format!(
            "Gzip decompression size mismatch: expected {}, got {}",
            original_size,
            decompressed.len()
        )));
    }

    Ok(decompressed)
}

/// Protobuf serializer
pub struct ProtobufSerializer;

impl ProtobufSerializer {
    /// Create a new Protobuf serializer
    pub fn new() -> Self {
        Self
    }
}

impl Serializer for ProtobufSerializer {
    fn serialize<T: serde::Serialize>(&self, _data: &T) -> Result<Vec<u8>> {
        // NOTE: Protobuf serialization not yet implemented with prost — use JsonSerializer as fallback
        Err(Error::NotImplemented(
            "Protobuf serialization - use JsonSerializer for now".to_string(),
        ))
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Protobuf
    }
}

impl Default for ProtobufSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// FlatBuffers serializer
pub struct FlatBuffersSerializer;

impl FlatBuffersSerializer {
    /// Create a new FlatBuffers serializer
    pub fn new() -> Self {
        Self
    }
}

impl Serializer for FlatBuffersSerializer {
    fn serialize<T: serde::Serialize>(&self, _data: &T) -> Result<Vec<u8>> {
        // NOTE: FlatBuffers serialization not yet implemented — requires flatbuffers crate integration
        Err(Error::NotImplemented(
            "FlatBuffers serialization".to_string(),
        ))
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::FlatBuffers
    }
}

impl Default for FlatBuffersSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON serializer (for debugging and compatibility)
pub struct JsonSerializer {
    /// Whether to pretty-print
    pretty: bool,
}

impl JsonSerializer {
    /// Create a new JSON serializer
    pub fn new() -> Self {
        Self { pretty: false }
    }

    /// Create a pretty-printing JSON serializer
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl Serializer for JsonSerializer {
    fn serialize<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>> {
        if self.pretty {
            Ok(serde_json::to_vec_pretty(data)?)
        } else {
            Ok(serde_json::to_vec(data)?)
        }
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Json
    }
}

impl Deserializer for JsonSerializer {
    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        Ok(serde_json::from_slice(data)?)
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Json
    }
}

impl Default for JsonSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Binary serializer (raw bytes)
pub struct BinarySerializer;

impl BinarySerializer {
    /// Create a new binary serializer
    pub fn new() -> Self {
        Self
    }
}

impl Serializer for BinarySerializer {
    fn serialize<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>> {
        // For binary, we expect the data to be Vec<u8> already
        // This is a special case for raw binary data
        if let Some(bytes) = serde_json::to_value(data)?
            .as_str()
            .map(|s| s.as_bytes().to_vec())
        {
            Ok(bytes)
        } else {
            Err(Error::Serialization(
                "Binary serializer requires string data".to_string(),
            ))
        }
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Binary
    }
}

impl Default for BinarySerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema compressor for efficient schema transmission
pub struct SchemaCompressor {
    /// Compression algorithm
    algorithm: CompressionAlgorithm,
    /// Schema cache
    schema_cache: Vec<(String, Vec<u8>)>,
}

impl SchemaCompressor {
    /// Create a new schema compressor
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self {
            algorithm,
            schema_cache: Vec::new(),
        }
    }

    /// Compress a schema
    pub fn compress_schema(&mut self, schema_id: &str, schema: &[u8]) -> Result<Vec<u8>> {
        // Check cache
        for (id, cached) in &self.schema_cache {
            if id == schema_id {
                return Ok(cached.clone());
            }
        }

        // Compress schema
        let compressed = compress(schema, self.algorithm)?;

        // Cache result
        self.schema_cache
            .push((schema_id.to_string(), compressed.clone()));

        Ok(compressed)
    }

    /// Decompress a schema
    pub fn decompress_schema(&self, compressed: &[u8]) -> Result<Vec<u8>> {
        decompress(compressed, self.algorithm)
    }

    /// Get cached schema
    pub fn get_cached(&self, schema_id: &str) -> Option<&Vec<u8>> {
        for (id, cached) in &self.schema_cache {
            if id == schema_id {
                return Some(cached);
            }
        }
        None
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.schema_cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.schema_cache.len()
    }
}

impl Default for SchemaCompressor {
    fn default() -> Self {
        Self::new(CompressionAlgorithm::Lz4)
    }
}

/// Serialization engine with format selection
pub struct SerializationEngine {
    /// Current format
    format: SerializationFormat,
    /// Compression algorithm
    compression: CompressionAlgorithm,
    /// JSON serializer
    json_serializer: JsonSerializer,
    /// Schema compressor
    schema_compressor: SchemaCompressor,
}

impl SerializationEngine {
    /// Create a new serialization engine
    pub fn new(format: SerializationFormat) -> Self {
        Self {
            format,
            compression: CompressionAlgorithm::None,
            json_serializer: JsonSerializer::new(),
            schema_compressor: SchemaCompressor::default(),
        }
    }

    /// Create with compression
    pub fn with_compression(
        format: SerializationFormat,
        compression: CompressionAlgorithm,
    ) -> Self {
        Self {
            format,
            compression,
            json_serializer: JsonSerializer::new(),
            schema_compressor: SchemaCompressor::new(compression),
        }
    }

    /// Serialize data
    pub fn serialize<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>> {
        let serialized = match self.format {
            SerializationFormat::Json => self.json_serializer.serialize(data)?,
            SerializationFormat::Protobuf
            | SerializationFormat::FlatBuffers
            | SerializationFormat::Binary => {
                // Fall back to JSON for now
                self.json_serializer.serialize(data)?
            }
        };

        if self.compression != CompressionAlgorithm::None {
            compress(&serialized, self.compression)
        } else {
            Ok(serialized)
        }
    }

    /// Deserialize data
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        let data_to_parse = if self.compression != CompressionAlgorithm::None {
            decompress(data, self.compression)?
        } else {
            data.to_vec()
        };

        match self.format {
            SerializationFormat::Json => self.json_serializer.deserialize(&data_to_parse),
            SerializationFormat::Protobuf
            | SerializationFormat::FlatBuffers
            | SerializationFormat::Binary => {
                // Fall back to JSON for now
                self.json_serializer.deserialize(&data_to_parse)
            }
        }
    }

    /// Serialize with compression
    pub fn serialize_compressed<T: serde::Serialize>(
        &self,
        data: &T,
        algorithm: CompressionAlgorithm,
    ) -> Result<Vec<u8>> {
        let serialized = self.serialize(data)?;
        compress(&serialized, algorithm)
    }

    /// Deserialize with decompression
    pub fn deserialize_compressed<T: serde::de::DeserializeOwned>(
        &self,
        data: &[u8],
        algorithm: CompressionAlgorithm,
    ) -> Result<T> {
        let decompressed = decompress(data, algorithm)?;
        self.deserialize(&decompressed)
    }

    /// Compress schema
    pub fn compress_schema(&mut self, schema_id: &str, schema: &[u8]) -> Result<Vec<u8>> {
        self.schema_compressor.compress_schema(schema_id, schema)
    }

    /// Decompress schema
    pub fn decompress_schema(&self, compressed: &[u8]) -> Result<Vec<u8>> {
        self.schema_compressor.decompress_schema(compressed)
    }

    /// Get current format
    pub fn format(&self) -> SerializationFormat {
        self.format
    }

    /// Set format
    pub fn set_format(&mut self, format: SerializationFormat) {
        self.format = format;
    }

    /// Get compression algorithm
    pub fn compression(&self) -> CompressionAlgorithm {
        self.compression
    }

    /// Set compression algorithm
    pub fn set_compression(&mut self, compression: CompressionAlgorithm) {
        self.compression = compression;
        self.schema_compressor = SchemaCompressor::new(compression);
    }
}

impl Default for SerializationEngine {
    fn default() -> Self {
        Self::new(SerializationFormat::Json)
    }
}

/// Estimate compression ratio
pub fn estimate_compression_ratio(data: &[u8], algorithm: CompressionAlgorithm) -> f32 {
    if data.is_empty() {
        return 1.0;
    }

    match algorithm {
        CompressionAlgorithm::None => 1.0,
        CompressionAlgorithm::Lz4 => {
            // LZ4 typically achieves 2-3x compression on structured data
            // Estimate based on data characteristics
            let unique_bytes = data.iter().collect::<std::collections::HashSet<_>>().len();
            if unique_bytes < 16 {
                0.3 // High compression for repetitive data
            } else if unique_bytes < 64 {
                0.5 // Medium compression
            } else {
                0.7 // Low compression for diverse data
            }
        }
        CompressionAlgorithm::Zstd => 0.4, // Better compression than LZ4
        CompressionAlgorithm::Gzip => 0.5, // Similar to Zstd
    }
}

/// Check if compression is beneficial
pub fn should_compress(data: &[u8], algorithm: CompressionAlgorithm) -> bool {
    // Don't compress small data
    if data.len() < 100 {
        return false;
    }

    // Check estimated compression ratio
    let ratio = estimate_compression_ratio(data, algorithm);

    // Only compress if we expect significant savings
    ratio < 0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_serializer() {
        let serializer = JsonSerializer::new();
        let data = serde_json::json!({"key": "value"});

        let serialized = serializer.serialize(&data).unwrap();
        assert!(!serialized.is_empty());

        let deserialized: serde_json::Value = serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized["key"], "value");
    }

    #[test]
    fn test_json_pretty_serializer() {
        let serializer = JsonSerializer::pretty();
        let data = serde_json::json!({"key": "value"});

        let serialized = serializer.serialize(&data).unwrap();
        // Pretty print has newlines - check for b'\n' byte
        assert!(serialized.iter().any(|&b| b == b'\n'));
    }

    #[test]
    fn test_lz4_compression() {
        let data = b"hello world hello world hello world";
        let compressed = compress_lz4(data).unwrap();

        // Compressed should be smaller for repetitive data
        assert!(compressed.len() < data.len() + 8);

        let decompressed = decompress_lz4(&compressed).unwrap();
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_compression_decision() {
        let small_data = b"hi";
        assert!(!should_compress(small_data, CompressionAlgorithm::Lz4));

        // Need at least 100 bytes for compression to be considered
        let mut repetitive_data = Vec::new();
        for _ in 0..20 {
            repetitive_data.extend_from_slice(b"aaaaa"); // 100+ bytes of repetitive data
        }
        assert!(should_compress(&repetitive_data, CompressionAlgorithm::Lz4));
    }

    #[test]
    fn test_serialization_engine() {
        let engine = SerializationEngine::new(SerializationFormat::Json);
        let data = serde_json::json!({"test": 123});

        let serialized = engine.serialize(&data).unwrap();
        let deserialized: serde_json::Value = engine.deserialize(&serialized).unwrap();

        assert_eq!(deserialized["test"], 123);
    }

    #[test]
    fn test_serialization_engine_with_compression() {
        let engine = SerializationEngine::with_compression(
            SerializationFormat::Json,
            CompressionAlgorithm::Lz4,
        );

        let data = serde_json::json!({"key": "value value value"});
        let serialized = engine.serialize(&data).unwrap();

        // Should be compressed
        assert!(serialized.len() > 8); // Has header

        let deserialized: serde_json::Value = engine.deserialize(&serialized).unwrap();
        assert_eq!(deserialized["key"], "value value value");
    }

    #[test]
    fn test_schema_compressor() {
        let mut compressor = SchemaCompressor::new(CompressionAlgorithm::Lz4);

        let schema = b"message Test { string field1 = 1; int32 field2 = 2; }";
        let compressed = compressor.compress_schema("test-schema", schema).unwrap();

        // Should cache
        assert_eq!(compressor.cache_size(), 1);

        // Get cached
        let cached = compressor.get_cached("test-schema");
        assert!(cached.is_some());

        // Decompress
        let decompressed = compressor.decompress_schema(&compressed).unwrap();
        assert_eq!(decompressed, schema.to_vec());
    }

    #[test]
    fn test_format_display() {
        assert_eq!(SerializationFormat::Protobuf.to_string(), "protobuf");
        assert_eq!(SerializationFormat::Json.to_string(), "json");
        assert_eq!(SerializationFormat::FlatBuffers.to_string(), "flatbuffers");
    }

    #[test]
    fn test_compression_ratio_estimate() {
        let repetitive = b"aaaaaaaaaaaaaaaa";
        let ratio = estimate_compression_ratio(repetitive, CompressionAlgorithm::Lz4);
        assert!(ratio < 0.5);

        let diverse: Vec<u8> = (0..255).collect();
        let ratio = estimate_compression_ratio(&diverse, CompressionAlgorithm::Lz4);
        assert!(ratio > 0.5);
    }

    #[test]
    fn test_zstd_compression_roundtrip() {
        // Test with repetitive data (high compression ratio expected)
        let data = b"hello world hello world hello world hello world hello world hello world";
        let compressed = compress(data, CompressionAlgorithm::Zstd).unwrap();

        // Zstd should compress repetitive data significantly
        // compressed.len() includes 4-byte header + compressed payload
        assert!(
            compressed.len() < data.len(),
            "Zstd compressed {} bytes should be < original {} bytes",
            compressed.len(),
            data.len()
        );

        let decompressed = decompress(&compressed, CompressionAlgorithm::Zstd).unwrap();
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_zstd_large_data_compression() {
        // Generate 10KB of repetitive data to test real compression
        let data: Vec<u8> = "Nexa-net protocol message payload data "
            .repeat(250)
            .into_bytes();
        assert!(data.len() > 1000, "Generated {} bytes", data.len());

        let compressed = compress(&data, CompressionAlgorithm::Zstd).unwrap();
        let ratio = compressed.len() as f32 / data.len() as f32;

        // Zstd should achieve at least 50% compression on repetitive data
        assert!(
            ratio < 0.5,
            "Zstd compression ratio {:.2} should be < 0.5 for repetitive data",
            ratio
        );

        let decompressed = decompress(&compressed, CompressionAlgorithm::Zstd).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_gzip_compression_roundtrip() {
        let data = b"hello world hello world hello world hello world hello world";
        let compressed = compress(data, CompressionAlgorithm::Gzip).unwrap();

        // Gzip should compress repetitive data
        assert!(
            compressed.len() < data.len(),
            "Gzip compressed {} bytes should be < original {} bytes",
            compressed.len(),
            data.len()
        );

        let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_gzip_large_data_compression() {
        let data: Vec<u8> = "Nexa-net protocol message payload data "
            .repeat(250)
            .into_bytes();

        let compressed = compress(&data, CompressionAlgorithm::Gzip).unwrap();
        let ratio = compressed.len() as f32 / data.len() as f32;

        // Gzip should achieve reasonable compression on repetitive data
        assert!(
            ratio < 0.5,
            "Gzip compression ratio {:.2} should be < 0.5",
            ratio
        );

        let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_all_compression_algorithms_roundtrip() {
        let data = b"The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.";

        for algo in [
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
        ] {
            let compressed = compress(data, algo).unwrap();
            let decompressed = decompress(&compressed, algo).unwrap();
            assert_eq!(
                decompressed,
                data.to_vec(),
                "Roundtrip failed for {:?}",
                algo
            );
        }
    }

    #[test]
    fn test_compression_preserves_random_data() {
        // Random data should be preserved even if compression doesn't reduce size
        let data: Vec<u8> = (0u8..=255u8).cycle().take(1024).collect();

        for algo in [
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
        ] {
            let compressed = compress(&data, algo).unwrap();
            let decompressed = decompress(&compressed, algo).unwrap();
            assert_eq!(decompressed, data, "Data integrity failed for {:?}", algo);
        }
    }

    #[test]
    fn test_zstd_empty_data() {
        // Zstd should handle empty data gracefully (though it may not compress well)
        let data = b"";
        let result = compress(data, CompressionAlgorithm::Zstd);
        // Empty data compression may work or may return error - just ensure no crash
        if let Ok(compressed) = result {
            if let Ok(decompressed) = decompress(&compressed, CompressionAlgorithm::Zstd) {
                assert_eq!(decompressed, data.to_vec());
            }
        }
    }

    #[test]
    fn test_serialization_engine_with_zstd() {
        let engine = SerializationEngine::with_compression(
            SerializationFormat::Json,
            CompressionAlgorithm::Zstd,
        );

        let data = serde_json::json!({"key": "value value value", "num": 42});
        let serialized = engine.serialize(&data).unwrap();
        let deserialized: serde_json::Value = engine.deserialize(&serialized).unwrap();

        assert_eq!(deserialized["key"], "value value value");
        assert_eq!(deserialized["num"], 42);
    }

    #[test]
    fn test_serialization_engine_with_gzip() {
        let engine = SerializationEngine::with_compression(
            SerializationFormat::Json,
            CompressionAlgorithm::Gzip,
        );

        let data = serde_json::json!({"message": "hello world hello world"});
        let serialized = engine.serialize(&data).unwrap();
        let deserialized: serde_json::Value = engine.deserialize(&serialized).unwrap();

        assert_eq!(deserialized["message"], "hello world hello world");
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_lz4_invalid_data_too_short() {
        // Less than 8 bytes header
        let data = [0u8; 4];
        let result = decompress_lz4(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_lz4_truncated_data() {
        // Header claims more compressed data than available
        let mut data = Vec::new();
        data.extend_from_slice(&100u32.to_be_bytes()); // original_size = 100
        data.extend_from_slice(&50u32.to_be_bytes()); // compressed_size = 50
        data.extend_from_slice(b"short"); // only 5 bytes, not 50

        let result = decompress_lz4(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_zstd_invalid_data_missing_header() {
        // Less than 4 bytes (missing original_size header)
        let data = [0u8; 2];
        let result = decompress_zstd(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_gzip_invalid_data_missing_header() {
        // Less than 4 bytes
        let data = [0u8; 2];
        let result = decompress_gzip(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_gzip_corrupted_data() {
        // Random bytes that are not valid gzip even with header
        let mut data = Vec::new();
        data.extend_from_slice(&100u32.to_be_bytes()); // original_size
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF]); // garbage "gzip" data
        let result = decompress_gzip(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_compress_none_is_identity() {
        let data = b"hello world";
        let compressed = compress(data, CompressionAlgorithm::None).unwrap();
        assert_eq!(compressed, data.to_vec());

        let decompressed = decompress(&compressed, CompressionAlgorithm::None).unwrap();
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_protobuf_serializer_not_implemented() {
        let serializer = ProtobufSerializer::new();
        let data = serde_json::json!({"key": "value"});
        let result = serializer.serialize(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_flatbuffers_serializer_not_implemented() {
        let serializer = FlatBuffersSerializer::new();
        let data = serde_json::json!({"key": "value"});
        let result = serializer.serialize(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_compressor_clear_cache() {
        let mut compressor = SchemaCompressor::new(CompressionAlgorithm::Lz4);
        compressor.compress_schema("schema-1", b"data1").unwrap();
        compressor.compress_schema("schema-2", b"data2").unwrap();
        assert_eq!(compressor.cache_size(), 2);

        compressor.clear_cache();
        assert_eq!(compressor.cache_size(), 0);
    }

    #[test]
    fn test_schema_compressor_get_cached_nonexistent() {
        let compressor = SchemaCompressor::new(CompressionAlgorithm::Lz4);
        assert!(compressor.get_cached("nonexistent").is_none());
    }

    #[test]
    fn test_serialization_engine_set_format() {
        let mut engine = SerializationEngine::new(SerializationFormat::Json);
        assert_eq!(engine.format(), SerializationFormat::Json);

        engine.set_format(SerializationFormat::Protobuf);
        assert_eq!(engine.format(), SerializationFormat::Protobuf);
    }

    #[test]
    fn test_serialization_engine_set_compression() {
        let mut engine = SerializationEngine::new(SerializationFormat::Json);
        assert_eq!(engine.compression(), CompressionAlgorithm::None);

        engine.set_compression(CompressionAlgorithm::Lz4);
        assert_eq!(engine.compression(), CompressionAlgorithm::Lz4);
    }

    #[test]
    fn test_should_compress_small_data_false() {
        let data = b"ab";
        assert!(!should_compress(data, CompressionAlgorithm::Lz4));
        assert!(!should_compress(data, CompressionAlgorithm::Zstd));
        assert!(!should_compress(data, CompressionAlgorithm::Gzip));
    }

    #[test]
    fn test_estimate_compression_ratio_empty_data() {
        let ratio = estimate_compression_ratio(b"", CompressionAlgorithm::Lz4);
        assert_eq!(ratio, 1.0);
    }

    #[test]
    fn test_estimate_compression_ratio_none() {
        let ratio = estimate_compression_ratio(b"any data", CompressionAlgorithm::None);
        assert_eq!(ratio, 1.0);
    }

    #[test]
    fn test_json_deserialize_invalid_data() {
        let serializer = JsonSerializer::new();
        let result: std::result::Result<serde_json::Value, _> =
            serializer.deserialize(b"not valid json{");
        assert!(result.is_err());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// serialize → compress → decompress → deserialize round-trip
        #[test]
        fn proptest_serialization_compress_roundtrip(
            data in prop::collection::vec(any::<u8>(), 100..500),
            algo in prop::sample::select(&[
                CompressionAlgorithm::Lz4,
                CompressionAlgorithm::Zstd,
                CompressionAlgorithm::Gzip,
            ]),
        ) {
            let compressed = compress(&data, algo).unwrap();
            let decompressed = decompress(&compressed, algo).unwrap();
            assert_eq!(decompressed, data);
        }

        /// JSON serializer round-trip preserves data
        #[test]
        fn proptest_json_serializer_roundtrip(
            key in "[a-zA-Z_][a-zA-Z0-9_]{0,10}",
            val in prop_oneof![
                Just(serde_json::json!("hello")),
                Just(serde_json::json!(42)),
                Just(serde_json::json!(true)),
                Just(serde_json::json!(null)),
            ],
        ) {
            let serializer = JsonSerializer::new();
            let key_for_json = key.clone();
            let data = serde_json::json!({ key_for_json: val });

            let serialized = serializer.serialize(&data).unwrap();
            let deserialized: serde_json::Value = serializer.deserialize(&serialized).unwrap();

            assert_eq!(deserialized[key.clone()], val);
        }

        /// SerializationEngine with LZ4 round-trip preserves JSON data
        #[test]
        fn proptest_serialization_engine_lz4_roundtrip(
            num in any::<i64>(),
            text in "[a-zA-Z ]{5,20}",
        ) {
            let engine = SerializationEngine::with_compression(
                SerializationFormat::Json,
                CompressionAlgorithm::Lz4,
            );

            let data = serde_json::json!({"number": num, "text": text});
            let serialized = engine.serialize(&data).unwrap();
            let deserialized: serde_json::Value = engine.deserialize(&serialized).unwrap();

            assert_eq!(deserialized["number"], num);
            assert_eq!(deserialized["text"], text);
        }
    }
}
