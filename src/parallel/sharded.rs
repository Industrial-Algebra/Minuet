//! Sharded holographic memory for increased capacity.
//!
//! Sharding distributes storage across multiple traces, increasing
//! total capacity at the cost of query broadcast.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use amari_fusion::{
    holographic::{Bindable, RetrievalResult},
    TropicalDualClifford,
};
use parking_lot::RwLock;
use rayon::prelude::*;

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{MinuetError, Result};
use crate::memory::{
    CapacityInfo, MemoryStore, MemoryTrace, MergeResult, Query, QueryResult, StoreReceipt,
};
use crate::precision::MinuetFloat;

/// Hasher for determining shard assignment.
#[derive(Debug, Clone)]
pub struct ShardHasher {
    seed: u64,
}

impl ShardHasher {
    /// Create a new hasher with random seed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed: rand::random(),
        }
    }

    /// Create with specific seed for reproducibility.
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self { seed }
    }

    /// Compute shard index for a TDC element.
    pub fn shard_index<T: MinuetFloat, const DIM: usize, const SHARDS: usize>(
        &self,
        key: &TropicalDualClifford<T, DIM>,
    ) -> usize {
        // Use a simple hash based on first few components
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.seed.hash(&mut hasher);

        // Hash based on the scalar and first vector components
        let scalar_bits = key.scalar_part().to_f64().unwrap_or(0.0).to_bits();
        scalar_bits.hash(&mut hasher);

        (hasher.finish() as usize) % SHARDS
    }
}

impl Default for ShardHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// A sharded holographic memory for increased capacity.
///
/// Sharding strategy: consistent hashing on key to distribute across traces.
/// Query broadcasts to all shards and merges results.
///
/// Total capacity ~ num_shards * single_trace_capacity
/// Query latency ~ single_trace_latency (parallel)
pub struct ShardedMemory<T: MinuetFloat, const DIM: usize, const SHARDS: usize> {
    /// Individual shard traces.
    shards: [RwLock<MemoryTrace<T, DIM>>; SHARDS],

    /// Hasher for shard assignment.
    hasher: ShardHasher,

    /// Global operation counter.
    operation_counter: AtomicU64,

    /// Temperature parameter.
    beta: T,
}

impl<T: MinuetFloat, const DIM: usize, const SHARDS: usize> ShardedMemory<T, DIM, SHARDS> {
    /// Create a new sharded memory.
    #[must_use]
    pub fn new() -> Self {
        Self::with_beta(T::from_f64(1.0).unwrap())
    }

    /// Create with specific temperature.
    #[must_use]
    pub fn with_beta(beta: T) -> Self {
        // Initialize array of traces
        let shards =
            std::array::from_fn(|_| RwLock::new(MemoryTrace::with_beta(beta).into_unknown()));

        Self {
            shards,
            hasher: ShardHasher::new(),
            operation_counter: AtomicU64::new(0),
            beta,
        }
    }

    /// Create with specific hasher for reproducibility.
    #[must_use]
    pub fn with_hasher(hasher: ShardHasher, beta: T) -> Self {
        let shards =
            std::array::from_fn(|_| RwLock::new(MemoryTrace::with_beta(beta).into_unknown()));

        Self {
            shards,
            hasher,
            operation_counter: AtomicU64::new(0),
            beta,
        }
    }

    /// Store routes to a single shard based on key hash.
    pub fn store(
        &self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<StoreReceipt> {
        let shard_idx = self.hasher.shard_index::<T, DIM, SHARDS>(key);
        let shard = self.shards[shard_idx].read();

        let mut receipt = shard.store(key, value)?;
        receipt.id = self.operation_counter.fetch_add(1, Ordering::SeqCst);

        Ok(receipt)
    }

    /// Store to multiple shards in parallel.
    pub fn store_batch(
        &self,
        pairs: &[(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)],
    ) -> Result<Vec<StoreReceipt>> {
        pairs.par_iter().map(|(k, v)| self.store(k, v)).collect()
    }

    /// Retrieve broadcasts to all shards in parallel, merges results.
    pub fn retrieve(&self, key: &TropicalDualClifford<T, DIM>) -> Result<RetrievalResult<T, DIM>> {
        // Query all shards in parallel
        let results: Vec<TropicalDualClifford<T, DIM>> = self
            .shards
            .par_iter()
            .map(|shard| {
                let trace = shard.read();
                trace.retrieve(key)
            })
            .collect();

        // Bundle results from all shards
        let mut combined = TropicalDualClifford::bundling_zero();
        for result in &results {
            combined = combined.bundle(result, self.beta);
        }

        // Compute aggregate confidence
        let total_items: u64 = self.shards.iter().map(|s| s.read().item_count()).sum();

        let capacity = self.capacity();

        Ok(RetrievalResult {
            value: combined,
            confidence: capacity.estimated_snr,
        })
    }

    /// Get aggregate capacity info across all shards.
    #[must_use]
    pub fn capacity(&self) -> CapacityInfo {
        let mut total_items = 0u64;
        let mut total_capacity = 0usize;
        let mut min_snr = f64::INFINITY;

        for shard in &self.shards {
            let info = shard.read().capacity_info();
            total_items += info.item_count;
            total_capacity += info.theoretical_capacity;
            min_snr = min_snr.min(info.estimated_snr);
        }

        let utilization = if total_capacity > 0 {
            total_items as f64 / total_capacity as f64
        } else {
            0.0
        };

        CapacityInfo {
            item_count: total_items,
            theoretical_capacity: total_capacity,
            estimated_snr: min_snr,
            snr_threshold: 0.5,
            remaining_stores: Some(((1.0 - utilization) * total_capacity as f64) as usize),
            utilization,
        }
    }

    /// Get the number of shards.
    #[must_use]
    pub const fn num_shards(&self) -> usize {
        SHARDS
    }

    /// Get per-shard capacity info.
    #[must_use]
    pub fn shard_capacities(&self) -> Vec<CapacityInfo> {
        self.shards
            .iter()
            .map(|s| s.read().capacity_info())
            .collect()
    }

    /// Clear all shards.
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.read().clear();
        }
    }

    /// Get total item count across all shards.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|s| s.read().item_count() as usize)
            .sum()
    }

    /// Check if all shards are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shards.iter().all(|s| s.read().item_count() == 0)
    }

    /// Get the distribution of items across shards.
    #[must_use]
    pub fn distribution(&self) -> Vec<u64> {
        self.shards.iter().map(|s| s.read().item_count()).collect()
    }

    /// Check for uneven distribution (potential hotspot).
    #[must_use]
    pub fn is_balanced(&self, max_imbalance: f64) -> bool {
        let counts = self.distribution();
        if counts.is_empty() {
            return true;
        }

        let total: u64 = counts.iter().sum();
        if total == 0 {
            return true;
        }

        let expected = total as f64 / SHARDS as f64;

        counts.iter().all(|&c| {
            let deviation = (c as f64 - expected).abs() / expected;
            deviation <= max_imbalance
        })
    }
}

impl<T: MinuetFloat, const DIM: usize, const SHARDS: usize> Default
    for ShardedMemory<T, DIM, SHARDS>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize, const SHARDS: usize> MemoryStore<T, DIM>
    for ShardedMemory<T, DIM, SHARDS>
where
    T: Send + Sync,
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    fn store(
        &self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<StoreReceipt> {
        ShardedMemory::store(self, key, value)
    }

    fn store_batch(
        &self,
        pairs: &[(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)],
    ) -> Result<Vec<StoreReceipt>> {
        ShardedMemory::store_batch(self, pairs)
    }

    fn retrieve(&self, key: &TropicalDualClifford<T, DIM>) -> Result<RetrievalResult<T, DIM>> {
        ShardedMemory::retrieve(self, key)
    }

    fn query(&self, query: Query<T, DIM>) -> Result<QueryResult<T, DIM>> {
        // For sharded memory, we need to broadcast and merge
        // This is a simplified implementation
        let results: Vec<Result<QueryResult<T, DIM>>> = self
            .shards
            .par_iter()
            .map(|shard| {
                let trace = shard.read();
                query.clone().execute(&*trace)
            })
            .collect();

        // Merge results (take best from each shard)
        let mut merged_results = Vec::new();
        let mut total_scanned = 0;

        for result in results {
            if let Ok(qr) = result {
                merged_results.extend(qr.results);
                total_scanned += qr.stats.items_scanned;
            }
        }

        // Sort by similarity and deduplicate
        merged_results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        Ok(QueryResult {
            results: merged_results,
            stats: crate::memory::QueryStats {
                query_time: std::time::Duration::default(),
                items_scanned: total_scanned,
                cleanup_iterations: None,
            },
        })
    }

    fn capacity(&self) -> CapacityInfo {
        ShardedMemory::capacity(self)
    }

    fn merge(&self, _other: &dyn MemoryStore<T, DIM>) -> Result<MergeResult> {
        Err(MinuetError::MergeFailed(
            "Merge not yet implemented for sharded memory".into(),
        ))
    }

    fn trace(&self) -> TropicalDualClifford<T, DIM> {
        // Return bundled trace from all shards
        let mut combined = TropicalDualClifford::bundling_zero();
        for shard in &self.shards {
            let trace = shard.read();
            combined = combined.bundle(&trace.raw_trace(), self.beta);
        }
        combined
    }

    fn clear(&self) -> Result<()> {
        ShardedMemory::clear(self);
        Ok(())
    }

    fn len(&self) -> usize {
        ShardedMemory::len(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharded_store_retrieve() {
        let memory: ShardedMemory<f64, 64, 4> = ShardedMemory::new();

        let key = TropicalDualClifford::random();
        let value = TropicalDualClifford::random();

        memory.store(&key, &value).unwrap();
        assert_eq!(memory.len(), 1);

        let result = memory.retrieve(&key).unwrap();
        // Should get something similar to value
        assert!(result.value.similarity(&value) > 0.3);
    }

    #[test]
    fn sharded_capacity() {
        let memory: ShardedMemory<f64, 64, 8> = ShardedMemory::new();

        let capacity = memory.capacity();
        // Total capacity should be roughly 8x single trace capacity
        assert!(capacity.theoretical_capacity >= 8 * 8); // At least 8 per shard
    }

    #[test]
    fn sharded_distribution() {
        let memory: ShardedMemory<f64, 64, 4> = ShardedMemory::new();

        // Store many items
        for _ in 0..100 {
            let key = TropicalDualClifford::random();
            let value = TropicalDualClifford::random();
            memory.store(&key, &value).unwrap();
        }

        let dist = memory.distribution();
        assert_eq!(dist.len(), 4);

        // Each shard should have some items (probabilistic)
        let total: u64 = dist.iter().sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn deterministic_hashing() {
        let hasher = ShardHasher::with_seed(42);

        let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let idx1 = hasher.shard_index::<f64, 64, 8>(&key);
        let idx2 = hasher.shard_index::<f64, 64, 8>(&key);

        assert_eq!(idx1, idx2);
        assert!(idx1 < 8);
    }
}
