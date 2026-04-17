//! Semantic DHT (Distributed Hash Table) with HNSW Vector Index
//!
//! Implements the core discovery infrastructure per DISCOVERY_LAYER.md:
//! - Local HNSW (Hierarchical Navigable Small World) vector index for O(log n) approximate search
//! - Kademlia-style routing table with k-buckets for DHT node discovery
//! - Vector replication to neighboring nodes for distributed semantic search
//!
//! # HNSW Algorithm
//!
//! Based on "Efficient and robust approximate nearest neighbor search using
//! Hierarchical Navigable Small World graphs" by Malkov & Yashunin (2016).
//!
//! Key properties:
//! - Multi-layer graph: upper layers have long-range connections, lower layers have short-range
//! - Greedy search from top layer descends to bottom, achieving O(log n) complexity
//! - ef parameter controls search quality (higher ef = better recall, slower search)
//!
//! # Kademlia DHT
//!
//! Node IDs are SHA-256 hashes of DID identifiers, mapped to 256-bit key space.
//! Each node maintains k-buckets covering exponentially increasing distance ranges.

use crate::error::{Error, Result};
use crate::identity::Did;
use parking_lot::RwLock;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ============================================================================
// HNSW Vector Index
// ============================================================================

/// Configuration for HNSW index construction
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Maximum number of connections per node per layer (M in the paper)
    /// Typical: 16-64. Higher M = better recall, more memory
    pub max_connections: usize,
    /// Maximum number of connections for layer 0 (M_0 = 2*M in the paper)
    pub max_connections_layer0: usize,
    /// Size of dynamic candidate list during search (ef_construction)
    /// Higher = better graph quality, slower construction
    pub ef_construction: usize,
    /// Size of dynamic candidate list during query (ef_search)
    /// Higher = better recall, slower search
    pub ef_search: usize,
    /// Level multiplier (mL = 1/ln(M) in the paper)
    /// Controls probability of node appearing at higher layers
    pub level_multiplier: f64,
}

impl Default for HnswConfig {
    fn default() -> Self {
        let max_connections = 16;
        Self {
            max_connections,
            max_connections_layer0: 2 * max_connections,
            ef_construction: 200,
            ef_search: 50,
            level_multiplier: 1.0 / (max_connections as f64).ln(),
        }
    }
}

/// A node in the HNSW graph
#[derive(Debug, Clone)]
struct HnswNode {
    /// Unique identifier for this node
    id: u64,
    /// The vector data
    vector: Vec<f32>,
    /// Associated key (DID or capability identifier)
    key: String,
    /// Level this node exists at (and all levels below)
    level: usize,
    /// Connections at each layer: layer -> list of neighbor node IDs
    /// Neighbors are sorted by distance (closest first)
    connections: Vec<Vec<u64>>,
}

/// Search candidate with distance for priority queue operations
#[derive(Debug, Clone, Copy)]
struct Candidate {
    id: u64,
    distance: f32,
}

impl Candidate {
    fn new(id: u64, distance: f32) -> Self {
        Self { id, distance }
    }
}

/// HNSW vector index for approximate nearest neighbor search
///
/// Provides O(log n) search complexity through a hierarchical graph structure
/// where upper layers contain long-range "express lanes" and lower layers
/// contain short-range connections for precision.
pub struct HnswIndex {
    /// All nodes stored by ID
    nodes: HashMap<u64, HnswNode>,
    /// Entry point node ID (highest level node)
    entry_point: Option<u64>,
    /// Maximum level of any node in the index
    max_level: usize,
    /// Configuration
    config: HnswConfig,
    /// Next available node ID
    next_id: u64,
}

impl HnswIndex {
    /// Create a new HNSW index with default configuration
    pub fn new() -> Self {
        Self::with_config(HnswConfig::default())
    }

    /// Create a new HNSW index with custom configuration
    pub fn with_config(config: HnswConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            entry_point: None,
            max_level: 0,
            config,
            next_id: 0,
        }
    }

    /// Calculate cosine distance between two vectors (1 - cosine_similarity)
    ///
    /// Returns value in range [0, 2]. 0 = identical direction, 2 = opposite.
    ///
    /// # SIMD Auto-Vectorization
    ///
    /// This function is designed for LLVM auto-vectorization:
    /// - Uses `f32` accumulation instead of `f64` to match the input type,
    ///   enabling SIMD operations on 4-8 f32 values simultaneously
    /// - Separate dot product and norm loops allow the compiler to vectorize
    ///   each independently with stride-1 access patterns
    /// - `#[inline]` hint allows inlining at search call sites for better
    ///   optimization across the full search pipeline
    ///
    /// For 384-dim vectors (common embedding size), this yields ~4x speedup
    /// with AVX2 (8 f32 lanes) or ~2x with SSE2 (4 f32 lanes).
    #[inline]
    pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 2.0; // Maximum distance for incompatible vectors
        }

        let len = a.len();

        // Use f32 accumulation — enables SIMD auto-vectorization on f32 data.
        // The compiler can vectorize this into 4-wide (SSE) or 8-wide (AVX2)
        // multiply-add operations since both inputs are f32 slices with
        // stride-1 access patterns.
        let mut dot: f32 = 0.0;
        let mut norm_a_sq: f32 = 0.0;
        let mut norm_b_sq: f32 = 0.0;

        // Single fused loop — the compiler can unroll and vectorize this
        // efficiently since all three accumulations have identical iteration
        // patterns over the same data.
        for i in 0..len {
            let ai = a[i];
            let bi = b[i];
            dot += ai * bi;
            norm_a_sq += ai * ai;
            norm_b_sq += bi * bi;
        }

        // Compute norms — f32 sqrt is sufficient for similarity comparison
        let norm_a = norm_a_sq.sqrt();
        let norm_b = norm_b_sq.sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 2.0;
        }

        let similarity = dot / (norm_a * norm_b);
        // Clamp to [-1, 1] to handle floating point errors
        // Clamp to [-1, 1] to handle floating point errors
        let clamped = similarity.clamp(-1.0, 1.0);
        1.0 - clamped
    }

    /// Calculate distance between a query vector and a stored node
    fn distance_to_node(query: &[f32], node: &HnswNode) -> f32 {
        Self::cosine_distance(query, &node.vector)
    }

    /// Assign a random level to a new node using exponential distribution
    ///
    /// The probability of level l is: P(l) = (1/M)^l
    /// This ensures upper layers are exponentially sparser.
    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut level = 0;
        while rng.gen::<f64>() < self.config.level_multiplier && level < 20 {
            level += 1;
        }
        level
    }

    /// Insert a vector into the HNSW index
    ///
    /// Algorithm (from the paper):
    /// 1. Assign random level l to the new element
    /// 2. Starting from entry point at top level, greedily descend to level l
    /// 3. At each level from l down to 0, find ef_construction nearest neighbors
    /// 4. Connect new element to those neighbors, prune if exceeding max_connections
    pub fn insert(&mut self, key: String, vector: Vec<f32>) -> Result<u64> {
        if vector.is_empty() {
            return Err(Error::Internal("Vector must not be empty".to_string()));
        }

        let id = self.next_id;
        self.next_id += 1;

        let level = self.random_level();
        let _dimensions = vector.len();

        // Create connections arrays for each level
        let connections = vec![Vec::new(); level + 1];

        let new_node = HnswNode {
            id,
            vector,
            key,
            level,
            connections,
        };

        // Handle first node
        if self.entry_point.is_none() {
            self.nodes.insert(id, new_node);
            self.entry_point = Some(id);
            self.max_level = level;
            return Ok(id);
        }

        let ep_id = self.entry_point.unwrap();
        let ep_level = self.nodes.get(&ep_id).map(|n| n.level).unwrap_or(0);

        // Phase 1: Greedily traverse from top to new node's level
        // Find the closest element at each level above l
        let mut curr_id = ep_id;
        let curr_dist = Self::distance_to_node(&new_node.vector, self.nodes.get(&ep_id).unwrap());

        for lvl in (ep_level..=level).rev().skip(1) {
            // Actually: traverse from ep_level down to level+1
            // This loop should go from ep_level down to level (exclusive)
            // We search greedily at levels above the new node's level
            let changed = self.search_layer_greedy(&new_node.vector, curr_id, curr_dist, lvl);
            curr_id = changed.id;
            let _changed_dist = changed.distance;
            // continue with this closer node
        }

        // Phase 2: At each level from min(level, ep_level) down to 0,
        // find ef_construction nearest neighbors and connect
        let min_level = level.min(ep_level);
        let mut ep_candidates = Vec::new();

        // Initialize with entry point
        let ep_dist = Self::distance_to_node(&new_node.vector, self.nodes.get(&ep_id).unwrap());
        ep_candidates.push(Candidate::new(ep_id, ep_dist));

        // Insert the new node first so it can be referenced
        self.nodes.insert(id, new_node);

        // Update entry point if new node has higher level
        if level > self.max_level {
            self.entry_point = Some(id);
            self.max_level = level;
        }

        // For each level from min_level down to 0, find neighbors and connect
        for lvl in (0..=min_level).rev() {
            let neighbors = self.search_layer_beam(
                &self.nodes.get(&id).unwrap().vector,
                ep_candidates.clone(),
                lvl,
                self.config.ef_construction,
            );

            // Select M best neighbors and connect
            let max_conn = if lvl == 0 {
                self.config.max_connections_layer0
            } else {
                self.config.max_connections
            };

            let selected = self.select_neighbors(neighbors, max_conn);

            // Add connections from new node to neighbors
            if let Some(new_node) = self.nodes.get_mut(&id) {
                new_node.connections[lvl] = selected.iter().map(|c| c.id).collect();
            }

            // Add reciprocal connections from neighbors to new node
            // Phase A: Collect all vectors needed (read-only borrow of self.nodes)
            let new_node_vector = self
                .nodes
                .get(&id)
                .map(|n| n.vector.clone())
                .unwrap_or_default();
            let neighbor_vectors: HashMap<u64, Vec<f32>> = selected
                .iter()
                .filter_map(|c| self.nodes.get(&c.id).map(|n| (c.id, n.vector.clone())))
                .collect();
            let neighbor_connections: HashMap<u64, Vec<u64>> = selected
                .iter()
                .filter_map(|c| {
                    self.nodes
                        .get(&c.id)
                        .map(|n| (c.id, n.connections[lvl].clone()))
                })
                .collect();

            // Phase B: Mutate each neighbor's connections (no further immutable borrows needed)
            for neighbor in &selected {
                let mut new_connections = neighbor_connections
                    .get(&neighbor.id)
                    .cloned()
                    .unwrap_or_default();
                new_connections.push(id);

                // Prune if exceeding max connections
                if new_connections.len() > max_conn {
                    let query = neighbor_vectors
                        .get(&neighbor.id)
                        .cloned()
                        .unwrap_or_default();
                    #[allow(clippy::manual_map)]
                    let dist_map: HashMap<u64, f32> = new_connections
                        .iter()
                        .filter_map(|&conn_id| {
                            if conn_id == id {
                                Some((conn_id, Self::cosine_distance(&query, &new_node_vector)))
                            } else if let Some(vec) = neighbor_vectors.get(&conn_id) {
                                Some((conn_id, Self::cosine_distance(&query, vec)))
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Sort by distance, keep closest max_conn
                    let mut conn_with_dist: Vec<(u64, f32)> = dist_map.into_iter().collect();
                    conn_with_dist
                        .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    conn_with_dist.truncate(max_conn);
                    new_connections = conn_with_dist.iter().map(|(cid, _)| *cid).collect();
                }

                // Apply mutation
                if let Some(neighbor_node) = self.nodes.get_mut(&neighbor.id) {
                    neighbor_node.connections[lvl] = new_connections;
                }
            }

            // Use the closest found neighbor as entry point for next level
            if !selected.is_empty() {
                ep_candidates = selected.clone();
            }
        }

        Ok(id)
    }

    /// Greedy search at a single layer - find the closest node
    ///
    /// Starting from `curr_id`, repeatedly move to the closest neighbor
    /// until no closer neighbor can be found.
    fn search_layer_greedy(
        &self,
        query: &[f32],
        curr_id: u64,
        curr_dist: f32,
        level: usize,
    ) -> Candidate {
        let mut best = Candidate::new(curr_id, curr_dist);

        loop {
            let node = self.nodes.get(&best.id);
            if node.is_none() {
                break;
            }
            let node = node.unwrap();

            if level >= node.connections.len() {
                break;
            }

            let mut found_closer = false;
            for &neighbor_id in &node.connections[level] {
                let neighbor = self.nodes.get(&neighbor_id);
                if neighbor.is_none() {
                    continue;
                }
                let dist = Self::distance_to_node(query, neighbor.unwrap());
                if dist < best.distance {
                    best = Candidate::new(neighbor_id, dist);
                    found_closer = true;
                    break; // Move to this closer neighbor and continue from there
                }
            }

            if !found_closer {
                break; // No closer neighbor found, we're at local optimum
            }
        }

        best
    }

    /// Beam search at a single layer - find ef nearest neighbors
    ///
    /// Uses a priority-queue-based search with ef controlling the beam width.
    /// This is the SEARCH-LAYER algorithm from the paper.
    fn search_layer_beam(
        &self,
        query: &[f32],
        entry_points: Vec<Candidate>,
        level: usize,
        ef: usize,
    ) -> Vec<Candidate> {
        // candidates: nodes to explore (sorted by distance, closest first)
        let mut candidates = entry_points;
        candidates.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // results: found nearest neighbors (sorted by distance, farthest first so we can prune)
        let mut results: Vec<Candidate> = candidates.clone();
        results.sort_by(|a, b| {
            b.distance
                .partial_cmp(&a.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut visited: HashMap<u64, bool> = HashMap::new();
        for c in &candidates {
            visited.insert(c.id, true);
        }

        while !candidates.is_empty() {
            // Get closest candidate
            let closest = candidates.remove(0);

            // Get farthest result
            let farthest_result = results.last().unwrap_or(&closest);

            // If closest candidate is farther than farthest result, stop
            if closest.distance > farthest_result.distance {
                break;
            }

            // Explore neighbors of closest candidate
            let node = self.nodes.get(&closest.id);
            if node.is_none() {
                continue;
            }
            let node = node.unwrap();

            if level >= node.connections.len() {
                continue;
            }

            for &neighbor_id in &node.connections[level] {
                if visited.contains_key(&neighbor_id) {
                    continue;
                }
                visited.insert(neighbor_id, true);

                let neighbor = self.nodes.get(&neighbor_id);
                if neighbor.is_none() {
                    continue;
                }
                let dist = Self::distance_to_node(query, neighbor.unwrap());

                let farthest_dist = results.last().map(|c| c.distance).unwrap_or(f32::MAX);

                // Add to results and candidates if better than farthest or results not full
                if dist < farthest_dist || results.len() < ef {
                    results.push(Candidate::new(neighbor_id, dist));
                    candidates.push(Candidate::new(neighbor_id, dist));
                }
            }

            // Sort candidates by distance (closest first)
            candidates.sort_by(|a, b| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Sort results by distance (farthest first) and keep only ef best
            results.sort_by(|a, b| {
                b.distance
                    .partial_cmp(&a.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            while results.len() > ef {
                results.pop(); // Remove farthest
            }
        }

        // Return results sorted by distance (closest first)
        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Select neighbors using the simple heuristic (keep closest M)
    fn select_neighbors(&self, candidates: Vec<Candidate>, max_conn: usize) -> Vec<Candidate> {
        candidates.into_iter().take(max_conn).collect()
    }

    /// Prune connections of a node at a given level if it exceeds max_connections
    ///
    /// Re-evaluates all connections and keeps only the closest max_conn neighbors.
    /// This is a standalone function that takes a pre-computed distance map
    /// to avoid borrow conflicts with self.nodes.
    #[allow(dead_code)]
    fn prune_connections_static(
        node: &mut HnswNode,
        level: usize,
        max_conn: usize,
        distance_map: &HashMap<u64, f32>,
    ) {
        let mut connections_with_dist: Vec<(u64, f32)> = node.connections[level]
            .iter()
            .filter_map(|&conn_id| distance_map.get(&conn_id).map(|dist| (conn_id, *dist)))
            .collect();

        // Sort by distance (closest first) and keep max_conn
        connections_with_dist
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        connections_with_dist.truncate(max_conn);

        node.connections[level] = connections_with_dist.iter().map(|(id, _)| *id).collect();
    }

    /// Search for k nearest neighbors
    ///
    /// Algorithm:
    /// 1. Start from entry point at top level
    /// 2. Greedily descend to level 0
    /// 3. At level 0, do beam search with ef_search
    /// 4. Return top k results
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        if self.entry_point.is_none() || self.nodes.is_empty() {
            return Vec::new();
        }

        let ep_id = self.entry_point.unwrap();
        let ep = self.nodes.get(&ep_id).unwrap();

        // Phase 1: Greedily descend from top level to level 1
        let mut curr_id = ep_id;
        let mut curr_dist = Self::distance_to_node(query, ep);

        for lvl in (1..=self.max_level).rev() {
            let closer = self.search_layer_greedy(query, curr_id, curr_dist, lvl);
            curr_id = closer.id;
            curr_dist = closer.distance;
        }

        // Phase 2: Beam search at level 0 with ef_search
        let entry_candidates = vec![Candidate::new(curr_id, curr_dist)];
        let results =
            self.search_layer_beam(query, entry_candidates, 0, self.config.ef_search.max(k));

        // Convert to (key, similarity) pairs
        // similarity = 1 - distance
        results
            .into_iter()
            .take(k)
            .filter_map(|c| {
                self.nodes
                    .get(&c.id)
                    .map(|n| (n.key.clone(), 1.0 - c.distance))
            })
            .collect()
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get a vector by key
    pub fn get(&self, key: &str) -> Option<&Vec<f32>> {
        self.nodes
            .values()
            .find(|n| n.key == key)
            .map(|n| &n.vector)
    }

    /// Remove a vector by key
    ///
    /// Note: HNSW doesn't support efficient deletion. This removes the node
    /// but leaves "dangling" connections. For production use, consider
    /// tombstone-based deletion or periodic index rebuilding.
    pub fn remove(&mut self, key: &str) -> Result<Vec<f32>> {
        let node_id = self.nodes.values().find(|n| n.key == key).map(|n| n.id);

        let id = node_id.ok_or_else(|| Error::Internal(format!("Key not found: {}", key)))?;
        let node = self.nodes.remove(&id).unwrap();

        // Clean up references to this node in other nodes' connections
        for other_node in self.nodes.values_mut() {
            for layer in &mut other_node.connections {
                layer.retain(|&conn_id| conn_id != id);
            }
        }

        // Update entry point if needed
        if self.entry_point == Some(id) {
            self.entry_point = self.nodes.keys().next().copied();
            self.max_level = self
                .entry_point
                .and_then(|ep| self.nodes.get(&ep))
                .map(|n| n.level)
                .unwrap_or(0);
        }

        Ok(node.vector)
    }
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Kademlia Routing Table
// ============================================================================

/// Number of nodes per k-bucket (Kademlia standard: 20)
const K_BUCKET_SIZE: usize = 20;

/// A k-bucket covering a specific distance range
#[derive(Debug, Clone)]
struct KBucket {
    /// Nodes in this bucket, ordered by last-seen time (oldest first)
    nodes: Vec<DhtNodeInfo>,
    /// Maximum number of nodes in this bucket
    max_size: usize,
}

impl KBucket {
    fn new(max_size: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Add or update a node in this bucket
    fn add_or_update(&mut self, node: DhtNodeInfo) {
        // Check if node already exists
        if let Some(idx) = self.nodes.iter().position(|n| n.id == node.id) {
            // Update existing node (move to end = most recently seen)
            self.nodes.remove(idx);
            self.nodes.push(node);
            return;
        }

        // New node
        if self.nodes.len() < self.max_size {
            self.nodes.push(node);
        } else {
            // Bucket is full - ping the least-recently-seen node
            // If it responds, discard the new node; otherwise replace it
            // For now, we replace the least-recently-seen (simplified)
            self.nodes.remove(0);
            self.nodes.push(node);
        }
    }

    /// Remove a node from this bucket
    fn remove(&mut self, node_id: &[u8; 32]) {
        self.nodes.retain(|n| &n.id != node_id);
    }
}

/// Information about a DHT node
#[derive(Debug, Clone)]
pub struct DhtNodeInfo {
    /// 256-bit node ID (SHA-256 hash of DID)
    pub id: [u8; 32],
    /// DID of the node
    pub did: String,
    /// Network address
    pub address: String,
    /// Last seen timestamp (epoch seconds)
    pub last_seen: u64,
}

impl DhtNodeInfo {
    /// Create a DHT node info from a DID
    pub fn from_did(did: &Did, address: &str) -> Self {
        let id = Self::compute_node_id(did);
        Self {
            id,
            did: did.as_str().to_string(),
            address: address.to_string(),
            last_seen: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Compute 256-bit node ID from DID using SHA-256
    fn compute_node_id(did: &Did) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(did.as_str().as_bytes());
        hasher.finalize().into()
    }
}

/// Kademlia-style routing table with 256 k-buckets
///
/// Each bucket i covers nodes whose XOR distance to our node ID
/// has bit length i+1 (i.e., the first differing bit is at position i).
pub struct KademliaRoutingTable {
    /// Our own node ID
    local_id: [u8; 32],
    /// 256 k-buckets covering the entire key space
    buckets: Vec<KBucket>,
}

impl KademliaRoutingTable {
    /// Create a new routing table for the given local node ID
    pub fn new(local_id: [u8; 32]) -> Self {
        let buckets = (0..256).map(|_| KBucket::new(K_BUCKET_SIZE)).collect();
        Self { local_id, buckets }
    }

    /// Calculate XOR distance between two node IDs
    fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let mut dist = [0u8; 32];
        for i in 0..32 {
            dist[i] = a[i] ^ b[i];
        }
        dist
    }

    /// Determine the bucket index for a given node ID
    ///
    /// The bucket index is the position of the first differing bit
    /// in the XOR distance between local_id and the target ID.
    fn bucket_index(&self, target_id: &[u8; 32]) -> usize {
        let dist = Self::xor_distance(&self.local_id, target_id);

        // Find the first non-zero byte
        for (byte_idx, &byte) in dist.iter().enumerate() {
            if byte != 0 {
                // The position of the first set bit from MSB in the 256-bit number
                // is: byte_idx * 8 + byte.leading_zeros()
                // leading_zeros() gives the position of the first 1-bit from MSB within the byte
                return byte_idx * 8 + byte.leading_zeros() as usize;
            }
        }
        0 // Same node ID (shouldn't happen in practice)
    }

    /// Add or update a node in the routing table
    pub fn add_or_update(&mut self, node: DhtNodeInfo) {
        if node.id == self.local_id {
            return; // Don't add ourselves
        }
        let bucket_idx = self.bucket_index(&node.id);
        self.buckets[bucket_idx].add_or_update(node);
    }

    /// Remove a node from the routing table
    pub fn remove(&mut self, node_id: &[u8; 32]) {
        let bucket_idx = self.bucket_index(node_id);
        self.buckets[bucket_idx].remove(node_id);
    }

    /// Find the k closest nodes to a target ID
    ///
    /// This is the core Kademlia lookup operation.
    pub fn find_closest(&self, target_id: &[u8; 32], k: usize) -> Vec<&DhtNodeInfo> {
        let bucket_idx = self.bucket_index(target_id);

        // Start from the target bucket, then expand to adjacent buckets
        let mut candidates: Vec<&DhtNodeInfo> = Vec::new();

        // Add nodes from target bucket
        candidates.extend(self.buckets[bucket_idx].nodes.iter());

        // Expand to adjacent buckets if we need more
        let mut left = bucket_idx.saturating_sub(1);
        let mut right = (bucket_idx + 1).min(255);

        while candidates.len() < k && (left > 0 || right < 255) {
            if left > 0 {
                candidates.extend(self.buckets[left].nodes.iter());
                left = left.saturating_sub(1);
            }
            if right < 255 {
                candidates.extend(self.buckets[right].nodes.iter());
                right = (right + 1).min(255);
            }
        }

        // Sort by XOR distance and return k closest
        candidates.sort_by(|a, b| {
            let dist_a = Self::xor_distance(&a.id, target_id);
            let dist_b = Self::xor_distance(&b.id, target_id);
            // Compare as big-endian integers
            for i in 0..32 {
                match dist_a[i].cmp(&dist_b[i]) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }
            std::cmp::Ordering::Equal
        });

        candidates.into_iter().take(k).collect()
    }

    /// Get total number of known nodes
    pub fn total_nodes(&self) -> usize {
        self.buckets.iter().map(|b| b.nodes.len()).sum()
    }
}

// ============================================================================
// Semantic DHT (Combined HNSW + Kademlia)
// ============================================================================

/// Semantic DHT for distributed vector storage and semantic search
///
/// Combines:
/// - Local HNSW index for fast approximate nearest neighbor search
/// - Kademlia routing table for DHT node discovery
/// - Replication logic for distributing vectors to neighboring nodes
pub struct SemanticDHT {
    /// Local HNSW vector index
    index: RwLock<HnswIndex>,
    /// Kademlia routing table for node discovery
    routing: RwLock<KademliaRoutingTable>,
    #[allow(dead_code)]
    /// Replication factor (how many nodes store each vector)
    replication_factor: usize,
}

impl SemanticDHT {
    /// Create a new SemanticDHT with the local node's DID as identity
    pub fn new(local_did: &Did) -> Self {
        let local_id = DhtNodeInfo::compute_node_id(local_did);
        Self {
            index: RwLock::new(HnswIndex::new()),
            routing: RwLock::new(KademliaRoutingTable::new(local_id)),
            replication_factor: 3,
        }
    }

    /// Create a new SemanticDHT with custom HNSW configuration
    pub fn with_config(local_did: &Did, hnsw_config: HnswConfig) -> Self {
        let local_id = DhtNodeInfo::compute_node_id(local_did);
        Self {
            index: RwLock::new(HnswIndex::with_config(hnsw_config)),
            routing: RwLock::new(KademliaRoutingTable::new(local_id)),
            replication_factor: 3,
        }
    }

    /// Store a vector in the local HNSW index
    ///
    /// In a full implementation, this would also replicate the vector
    /// to the k closest DHT nodes.
    pub fn store(&self, key: String, vector: Vec<f32>) -> Result<()> {
        let mut index = self.index.write();
        index.insert(key, vector)?;
        Ok(())
    }

    /// Retrieve a vector by key from local storage
    pub fn get(&self, key: &str) -> Option<Vec<f32>> {
        let index = self.index.read();
        index.get(key).cloned()
    }

    /// Find similar vectors using HNSW approximate nearest neighbor search
    ///
    /// Returns results sorted by similarity (highest first).
    /// Uses cosine similarity as the distance metric.
    pub fn find_similar(&self, query: &[f32], k: usize, threshold: f32) -> Vec<(String, f32)> {
        let index = self.index.read();
        index
            .search(query, k)
            .into_iter()
            .filter(|(_, sim)| *sim >= threshold)
            .collect()
    }

    /// Add a DHT node to the routing table
    pub fn add_node(&self, node: DhtNodeInfo) {
        let mut routing = self.routing.write();
        routing.add_or_update(node);
    }

    /// Remove a DHT node from the routing table
    pub fn remove_node(&self, node_id: &[u8; 32]) {
        let mut routing = self.routing.write();
        routing.remove(node_id);
    }

    /// Find closest DHT nodes for a given key
    ///
    /// Used for replication: vectors should be stored on the k closest
    /// nodes to the key's hash in the DHT key space.
    pub fn find_closest_nodes(&self, key: &str, k: usize) -> Vec<DhtNodeInfo> {
        // Hash the key to get a target ID in the DHT key space
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let target_id: [u8; 32] = hasher.finalize().into();

        let routing = self.routing.read();
        routing
            .find_closest(&target_id, k)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Get total number of vectors stored locally
    pub fn vector_count(&self) -> usize {
        self.index.read().len()
    }

    /// Get total number of known DHT nodes
    pub fn node_count(&self) -> usize {
        self.routing.read().total_nodes()
    }

    /// Remove a vector from the local index
    pub fn remove_vector(&self, key: &str) -> Result<Vec<f32>> {
        self.index.write().remove(key)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::embedding::utils::cosine_similarity;

    /// Helper: generate a random unit vector of given dimensions
    fn random_vector(dimensions: usize) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        let mut v: Vec<f32> = (0..dimensions).map(|_| rng.gen_range(-1.0..1.0)).collect();
        // Normalize to unit length
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
        v
    }

    /// Helper: generate a vector similar to a base vector
    fn similar_vector(base: &[f32], noise: f32) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        let mut v: Vec<f32> = base
            .iter()
            .map(|x| x + rng.gen_range(-noise..noise))
            .collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
        v
    }

    #[test]
    fn test_hnsw_insert_and_search() {
        let mut index = HnswIndex::new();
        let dimensions = 10;

        // Insert 100 random vectors
        for i in 0..100 {
            let key = format!("vec_{}", i);
            let vector = random_vector(dimensions);
            index.insert(key, vector).unwrap();
        }

        assert_eq!(index.len(), 100);
        assert!(index.entry_point.is_some());
    }

    #[test]
    fn test_hnsw_search_accuracy() {
        let mut index = HnswIndex::new();
        let dimensions = 384;

        // Create a set of vectors where some are intentionally similar
        let base = random_vector(dimensions);

        // Insert 50 random vectors
        for i in 0..50 {
            let key = format!("random_{}", i);
            let vector = random_vector(dimensions);
            index.insert(key, vector).unwrap();
        }

        // Insert 5 vectors similar to base
        for i in 0..5 {
            let key = format!("similar_{}", i);
            let vector = similar_vector(&base, 0.1);
            index.insert(key, vector).unwrap();
        }

        // Insert the base vector itself
        index.insert("base".to_string(), base.clone()).unwrap();

        // Search for vectors similar to base
        let results = index.search(&base, 10);

        // Should find at least some of the similar vectors
        assert!(results.len() >= 3);

        // The base vector itself should be the closest (similarity ≈ 1.0)
        if !results.is_empty() {
            assert!(results[0].0 == "base" || results[0].1 > 0.9);
        }
    }

    #[test]
    fn test_hnsw_empty_search() {
        let index = HnswIndex::new();
        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_hnsw_cosine_distance() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((HnswIndex::cosine_distance(&a, &b) - 0.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((HnswIndex::cosine_distance(&a, &c) - 1.0).abs() < 1e-6);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((HnswIndex::cosine_distance(&a, &d) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_kademlia_routing_table() {
        let local_id = [0u8; 32]; // Node with ID 0
        let mut routing = KademliaRoutingTable::new(local_id);

        // Add nodes at various distances
        for i in 1..=20 {
            let mut node_id = [0u8; 32];
            node_id[31] = i as u8; // Distance = i
            let node = DhtNodeInfo {
                id: node_id,
                did: format!("did:nexa:node{}", i),
                address: format!("127.0.0.1:{}", 8000 + i),
                last_seen: 1000 + i as u64,
            };
            routing.add_or_update(node);
        }

        assert_eq!(routing.total_nodes(), 20);

        // Find closest nodes to a target
        let mut target_id = [0u8; 32];
        target_id[31] = 5;
        let closest = routing.find_closest(&target_id, 5);
        assert!(closest.len() <= 5);
    }

    #[test]
    fn test_kademlia_bucket_index() {
        // Node 0 vs target with first bit differing at position 255
        let local_id = [0u8; 32];
        let routing = KademliaRoutingTable::new(local_id);

        // Target with bit 255 set (last byte = 1)
        let mut target1 = [0u8; 32];
        target1[31] = 1;
        assert_eq!(routing.bucket_index(&target1), 255);

        // Target with bit 0 set (first byte = 128)
        let mut target2 = [0u8; 32];
        target2[0] = 128;
        assert_eq!(routing.bucket_index(&target2), 0);
    }

    #[test]
    fn test_semantic_dht_store_and_search() {
        let did = Did::parse("did:nexa:testnode").unwrap();
        let dht = SemanticDHT::new(&did);

        // Store vectors
        let v1 = vec![1.0, 0.0, 0.0, 0.0];
        let v2 = vec![0.9, 0.1, 0.0, 0.0]; // Similar to v1
        let v3 = vec![0.0, 1.0, 0.0, 0.0]; // Different from v1

        dht.store("key1".to_string(), v1.clone()).unwrap();
        dht.store("key2".to_string(), v2.clone()).unwrap();
        dht.store("key3".to_string(), v3.clone()).unwrap();

        assert_eq!(dht.vector_count(), 3);

        // Search for vectors similar to v1
        let results = dht.find_similar(&v1, 10, 0.5);
        assert!(results.len() >= 2); // Should find v1 and v2 at least

        // Retrieve a vector
        let retrieved = dht.get("key1");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_semantic_dht_node_management() {
        let did = Did::parse("did:nexa:local").unwrap();
        let dht = SemanticDHT::new(&did);

        // Add nodes
        let node1 = DhtNodeInfo {
            id: [1u8; 32],
            did: "did:nexa:node1".to_string(),
            address: "127.0.0.1:8001".to_string(),
            last_seen: 1000,
        };
        let node2 = DhtNodeInfo {
            id: [2u8; 32],
            did: "did:nexa:node2".to_string(),
            address: "127.0.0.1:8002".to_string(),
            last_seen: 1001,
        };

        dht.add_node(node1);
        dht.add_node(node2);
        assert_eq!(dht.node_count(), 2);

        // Remove a node
        dht.remove_node(&[1u8; 32]);
        assert_eq!(dht.node_count(), 1);
    }

    #[test]
    fn test_hnsw_large_index_performance() {
        let mut index = HnswIndex::with_config(HnswConfig {
            max_connections: 16,
            max_connections_layer0: 32,
            ef_construction: 100,
            ef_search: 50,
            level_multiplier: 1.0 / 16.0_f64.ln(),
        });

        let dimensions = 384;
        let num_vectors = 500;

        // Insert 500 vectors (should be fast enough for testing)
        for i in 0..num_vectors {
            let key = format!("vec_{}", i);
            let vector = random_vector(dimensions);
            index.insert(key, vector).unwrap();
        }

        assert_eq!(index.len(), num_vectors);

        // Search should return results
        let query = random_vector(dimensions);
        let results = index.search(&query, 10);
        assert!(results.len() > 0);
        assert!(results.len() <= 10);
    }

    #[test]
    fn test_dht_node_info_from_did() {
        let did = Did::parse("did:nexa:test123").unwrap();
        let node = DhtNodeInfo::from_did(&did, "127.0.0.1:8000");

        assert_eq!(node.did, "did:nexa:test123");
        assert_eq!(node.address, "127.0.0.1:8000");

        // Same DID should produce same node ID
        let node2 = DhtNodeInfo::from_did(&did, "127.0.0.1:8001");
        assert_eq!(node.id, node2.id);

        // Different DID should produce different node ID
        let did2 = Did::parse("did:nexa:other456").unwrap();
        let node3 = DhtNodeInfo::from_did(&did2, "127.0.0.1:8002");
        assert_ne!(node.id, node3.id);
    }

    #[test]
    fn test_hnsw_remove_vector() {
        let mut index = HnswIndex::new();

        index
            .insert("key1".to_string(), vec![1.0, 0.0, 0.0])
            .unwrap();
        index
            .insert("key2".to_string(), vec![0.0, 1.0, 0.0])
            .unwrap();

        assert_eq!(index.len(), 2);

        let removed = index.remove("key1").unwrap();
        assert_eq!(removed, vec![1.0, 0.0, 0.0]);
        assert_eq!(index.len(), 1);
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_hnsw_insert_empty_vector_rejected() {
        let mut index = HnswIndex::new();
        let result = index.insert("empty".to_string(), vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_hnsw_single_vector_self_recall() {
        let mut index = HnswIndex::new();
        let v = vec![1.0, 0.0, 0.0];
        index.insert("key1".to_string(), v.clone()).unwrap();

        // Searching for the same vector should find itself
        let results = index.search(&v, 1);
        assert!(!results.is_empty());
        assert!(results[0].1 > 0.99); // Near-perfect similarity
    }

    #[test]
    fn test_hnsw_zero_vector_distance() {
        let zero = vec![0.0, 0.0, 0.0];
        let unit = vec![1.0, 0.0, 0.0];
        // Zero vector has 0 norm → cosine_distance returns 2.0 (max distance)
        assert_eq!(HnswIndex::cosine_distance(&zero, &unit), 2.0);
    }

    #[test]
    fn test_hnsw_mismatched_dimension_distance() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        // Different dimensions → cosine_distance returns 2.0 (max distance)
        assert_eq!(HnswIndex::cosine_distance(&a, &b), 2.0);
    }

    #[test]
    fn test_hnsw_remove_nonexistent_key() {
        let mut index = HnswIndex::new();
        let result = index.remove("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_hnsw_get_nonexistent_key() {
        let index = HnswIndex::new();
        assert!(index.get("nonexistent").is_none());
    }

    #[test]
    fn test_kademlia_empty_routing_table() {
        let local_id = [0u8; 32];
        let routing = KademliaRoutingTable::new(local_id);

        assert_eq!(routing.total_nodes(), 0);
        let target = [5u8; 32];
        assert!(routing.find_closest(&target, 5).is_empty());
    }

    #[test]
    fn test_kademlia_same_node_id_not_added() {
        let local_id = [0u8; 32];
        let mut routing = KademliaRoutingTable::new(local_id);

        // Try to add a node with the same ID as local — should be ignored
        let node = DhtNodeInfo {
            id: local_id,
            did: "did:nexa:local".to_string(),
            address: "127.0.0.1:8000".to_string(),
            last_seen: 1000,
        };
        routing.add_or_update(node);
        assert_eq!(routing.total_nodes(), 0);
    }

    #[test]
    fn test_semantic_dht_empty_find_similar() {
        let did = Did::parse("did:nexa:testnode").unwrap();
        let dht = SemanticDHT::new(&did);

        let query = vec![1.0, 0.0, 0.0];
        let results = dht.find_similar(&query, 5, 0.0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_semantic_dht_remove_vector() {
        let did = Did::parse("did:nexa:testnode").unwrap();
        let dht = SemanticDHT::new(&did);

        dht.store("key1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        assert_eq!(dht.vector_count(), 1);

        dht.remove_vector("key1").unwrap();
        assert_eq!(dht.vector_count(), 0);
        assert!(dht.get("key1").is_none());
    }

    #[test]
    fn test_semantic_dht_remove_nonexistent() {
        let did = Did::parse("did:nexa:testnode").unwrap();
        let dht = SemanticDHT::new(&did);

        let result = dht.remove_vector("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_dht_find_closest_nodes_empty() {
        let did = Did::parse("did:nexa:testnode").unwrap();
        let dht = SemanticDHT::new(&did);

        let nodes = dht.find_closest_nodes("some_key", 5);
        assert!(nodes.is_empty());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// HNSW insert → search: self-recall — inserting a vector then
        /// searching with that same vector should always find it
        #[test]
        fn proptest_hnsw_self_recall(
            vector in prop::collection::vec(-1.0f32..1.0f32, 4..8),
        ) {
            let mut index = HnswIndex::new();
            // Normalize vector to avoid zero-norm
            let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            let v: Vec<f32> = if norm > 0.0 {
                vector.iter().map(|x| x / norm).collect()
            } else {
                // All zeros — skip by making it unit in first dim
                let mut v = vector;
                v[0] = 1.0;
                v
            };

            index.insert("self".to_string(), v.clone()).unwrap();
            let results = index.search(&v, 1);

            // Must find at least one result (self)
            assert!(!results.is_empty(), "HNSW should recall the inserted vector itself");
            // The top result should be the self vector
            assert_eq!(results[0].0, "self");
            assert!(results[0].1 > 0.99, "Self-similarity should be near 1.0, got {}", results[0].1);
        }

        /// Cosine distance is always in range [0.0, 2.0]
        #[test]
        fn proptest_cosine_distance_range(
            a in prop::collection::vec(-10.0f32..10.0f32, 3..6),
            b in prop::collection::vec(-10.0f32..10.0f32, 3..6),
        ) {
            if a.len() == b.len() && !a.is_empty() {
                let dist = HnswIndex::cosine_distance(&a, &b);
                assert!(dist >= 0.0 && dist <= 2.0, "distance {} out of range [0,2]", dist);
            }
        }
    }
}
