// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Sharded memory store for larger capacity.
//!
//! Distributes items across multiple traces via consistent hashing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use amari_holographic::BindingAlgebra;
use parking_lot::RwLock;

use crate::error::MinuetResult;
use crate::traits::{
    CapacityInfo, CapacityWarning, MemoryStore, MemoryTrace, RetrievalResult, StoreOptions,
    StoreReceipt, TraceCapacityInfo,
};

use super::DenseTrace;

/// A sharded holographic memory store.
///
/// Distributes items across multiple traces via consistent hashing.
/// Total capacity ≈ shards × single_trace_capacity.
///
/// # Shard Selection
///
/// Items are assigned to shards based on key hash. This means:
/// - Same key always goes to same shard
/// - Retrieval only needs to check one shard
/// - Load is distributed across shards
///
/// # Example
///
/// ```rust
/// use minuet::store::ShardedStore;
/// use amari_holographic::ProductCliffordAlgebra;
///
/// // 8 shards × ~80 capacity each = ~640 total
/// let store = ShardedStore::<ProductCliffordAlgebra<64>>::with_shards(8);
/// ```
pub struct ShardedStore<A: BindingAlgebra> {
    shards: Vec<RwLock<DenseTrace<A>>>,
    next_id: AtomicU64,
    config: ShardedStoreConfig,
}

/// Configuration for ShardedStore.
#[derive(Clone, Debug)]
pub struct ShardedStoreConfig {
    /// Number of shards.
    pub num_shards: usize,
    /// Temperature for bundling operations.
    pub bundle_temperature: f64,
    /// Whether to search all shards on retrieval (slower but more robust).
    pub broadcast_retrieval: bool,
    /// Warning threshold (utilization fraction).
    pub warning_threshold: f64,
}

impl Default for ShardedStoreConfig {
    fn default() -> Self {
        Self {
            num_shards: 8,
            bundle_temperature: 1.0,
            broadcast_retrieval: false,
            warning_threshold: 0.8,
        }
    }
}

impl<A: BindingAlgebra> ShardedStore<A> {
    /// Create with default number of shards (8).
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ShardedStoreConfig::default())
    }

    /// Create with specified number of shards.
    #[must_use]
    pub fn with_shards(num_shards: usize) -> Self {
        Self::with_config(ShardedStoreConfig {
            num_shards,
            ..Default::default()
        })
    }

    /// Create with configuration.
    #[must_use]
    pub fn with_config(config: ShardedStoreConfig) -> Self {
        let shards = (0..config.num_shards)
            .map(|_| RwLock::new(DenseTrace::with_temperature(config.bundle_temperature)))
            .collect();

        Self {
            shards,
            next_id: AtomicU64::new(0),
            config,
        }
    }

    /// Get the number of shards.
    #[must_use]
    pub fn num_shards(&self) -> usize {
        self.shards.len()
    }

    /// Determine which shard a key belongs to.
    fn shard_for_key(&self, key: &A) -> usize {
        // Hash the key's coefficients
        let coeffs = key.to_coefficients();
        let mut hasher = DefaultHasher::new();
        for c in coeffs {
            c.to_bits().hash(&mut hasher);
        }
        (hasher.finish() as usize) % self.shards.len()
    }

    #[allow(clippy::unnecessary_wraps)]
    fn retrieve_targeted(&self, key: &A) -> MinuetResult<RetrievalResult<A>> {
        let shard_idx = self.shard_for_key(key);
        let trace = self.shards[shard_idx].read();
        let raw = trace.unbind(key);
        let confidence = (trace.estimated_snr() / 10.0).min(1.0);

        Ok(RetrievalResult {
            value: raw,
            confidence,
            attribution: vec![],
        })
    }

    #[allow(clippy::unnecessary_wraps)]
    fn retrieve_broadcast(&self, key: &A) -> MinuetResult<RetrievalResult<A>> {
        // Search all shards, return best match by SNR
        let results: Vec<_> = self
            .shards
            .iter()
            .map(|shard| {
                let trace = shard.read();
                let raw = trace.unbind(key);
                let confidence = (trace.estimated_snr() / 10.0).min(1.0);
                (raw, confidence)
            })
            .collect();

        // Return highest confidence result
        let (value, confidence) = results
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| (A::zero(), 0.0));

        Ok(RetrievalResult {
            value,
            confidence,
            attribution: vec![],
        })
    }
}

impl<A: BindingAlgebra> Default for ShardedStore<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> MemoryStore for ShardedStore<A> {
    type Trace = DenseTrace<A>;
    type Algebra = A;

    fn store(&self, key: &A, value: &A) -> MinuetResult<StoreReceipt> {
        self.store_with_options(key, value, StoreOptions::new())
    }

    fn store_with_options(
        &self,
        key: &A,
        value: &A,
        options: StoreOptions,
    ) -> MinuetResult<StoreReceipt> {
        let shard_idx = self.shard_for_key(key);
        let bound = key.bind(value);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut trace = self.shards[shard_idx].write();
        trace.add(&bound, options.weight);

        let snr = trace.estimated_snr();
        let utilization = trace.utilization();

        let warning = if utilization > self.config.warning_threshold {
            Some(CapacityWarning::ApproachingCapacity { utilization })
        } else {
            None
        };

        Ok(StoreReceipt {
            id,
            post_snr: snr,
            warning,
            location: format!("shard_{shard_idx}"),
        })
    }

    fn store_batch(&self, pairs: &[(A, A)]) -> MinuetResult<Vec<StoreReceipt>> {
        pairs.iter().map(|(k, v)| self.store(k, v)).collect()
    }

    fn retrieve(&self, key: &A) -> MinuetResult<RetrievalResult<A>> {
        if self.config.broadcast_retrieval {
            self.retrieve_broadcast(key)
        } else {
            self.retrieve_targeted(key)
        }
    }

    fn capacity_info(&self) -> CapacityInfo {
        let per_trace: Vec<_> = self
            .shards
            .iter()
            .enumerate()
            .map(|(i, shard)| {
                let trace = shard.read();
                TraceCapacityInfo {
                    name: format!("shard_{i}"),
                    items: trace.item_count(),
                    capacity: trace.theoretical_capacity(),
                    utilization: trace.utilization(),
                }
            })
            .collect();

        let total_items: usize = per_trace.iter().map(|t| t.items).sum();
        let total_capacity: usize = per_trace.iter().map(|t| t.capacity).sum();
        let utilization = if total_capacity > 0 {
            total_items as f64 / total_capacity as f64
        } else {
            0.0
        };

        CapacityInfo {
            total_items,
            theoretical_capacity: total_capacity,
            utilization,
            estimated_snr: if total_items > 0 {
                (total_capacity as f64 / total_items as f64).sqrt()
            } else {
                f64::INFINITY
            },
            per_trace,
        }
    }

    fn clear(&self) -> MinuetResult<()> {
        for shard in &self.shards {
            shard.write().clear();
        }
        Ok(())
    }

    fn trace_count(&self) -> usize {
        self.shards.len()
    }

    fn total_items(&self) -> usize {
        self.shards.iter().map(|s| s.read().item_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>; // 64 dimensions

    #[test]
    fn sharded_store_creation() {
        let store = ShardedStore::<TestAlgebra>::with_shards(4);
        assert_eq!(store.num_shards(), 4);
        assert_eq!(store.trace_count(), 4);
    }

    #[test]
    fn store_and_retrieve() {
        let store = ShardedStore::<TestAlgebra>::with_shards(4);

        let key = TestAlgebra::random_versor(2);
        let value = TestAlgebra::random_versor(2);

        store.store(&key, &value).unwrap();

        let result = store.retrieve(&key).unwrap();
        let sim = result.value.similarity(&value);
        assert!(sim > 0.5, "similarity was {}", sim);
    }

    #[test]
    fn capacity_scales_with_shards() {
        let store_4 = ShardedStore::<TestAlgebra>::with_shards(4);
        let store_8 = ShardedStore::<TestAlgebra>::with_shards(8);

        let cap_4 = store_4.capacity_info().theoretical_capacity;
        let cap_8 = store_8.capacity_info().theoretical_capacity;

        // 8 shards should have ~2x capacity of 4 shards
        assert!(cap_8 > cap_4, "8 shards: {}, 4 shards: {}", cap_8, cap_4);
    }
}
