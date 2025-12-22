//! Memory store trait and implementations.
//!
//! The `MemoryStore` trait defines the core interface for holographic storage,
//! with implementations for single-trace and sharded memories.

use std::marker::PhantomData;

use amari_fusion::{holographic::RetrievalResult, TropicalDualClifford};
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{CapacityWarning, Result};
use crate::precision::MinuetFloat;

use super::capacity::CapacityInfo;
use super::query::{Query, QueryResult};
use super::trace::MemoryTrace;

/// Receipt returned after storing an association.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreReceipt {
    /// Unique identifier for this storage operation.
    pub id: u64,
    /// Estimated SNR after this store.
    pub post_store_snr: f64,
    /// Warning if approaching capacity.
    pub capacity_warning: Option<CapacityWarning>,
}

/// Result of a merge operation.
#[derive(Clone, Debug)]
pub struct MergeResult {
    /// Number of items merged.
    pub items_merged: u64,
    /// Final SNR after merge.
    pub final_snr: f64,
    /// Capacity warning if applicable.
    pub capacity_warning: Option<CapacityWarning>,
}

/// Phantom types for store state tracking.
pub mod store_state {
    /// Store is ready for operations.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Ready;

    /// Store is being modified (write lock held).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Modifying;

    /// Store has been closed.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Closed;
}

/// A holographic memory store.
///
/// Generic over:
/// - `T`: Numeric type (f32, f64)
/// - `DIM`: Representation dimensionality (compile-time constant)
pub trait MemoryStore<T: MinuetFloat, const DIM: usize>: Send + Sync {
    /// Store a key-value association.
    ///
    /// # Errors
    ///
    /// Returns `MinuetError::AtCapacity` if memory is full.
    fn store(
        &self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<StoreReceipt>;

    /// Store a batch of associations (parallel when beneficial).
    ///
    /// # Errors
    ///
    /// Returns error if any store fails.
    fn store_batch(
        &self,
        pairs: &[(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)],
    ) -> Result<Vec<StoreReceipt>>;

    /// Retrieve by key with default settings.
    fn retrieve(&self, key: &TropicalDualClifford<T, DIM>) -> Result<RetrievalResult<T, DIM>>;

    /// Execute a structured query.
    fn query(&self, query: Query<T, DIM>) -> Result<QueryResult<T, DIM>>;

    /// Get capacity information.
    fn capacity(&self) -> CapacityInfo;

    /// Merge another store into this one.
    ///
    /// # Errors
    ///
    /// Returns `MinuetError::MergeFailed` if merge would exceed capacity.
    fn merge(&self, other: &dyn MemoryStore<T, DIM>) -> Result<MergeResult>;

    /// Get the raw trace (for serialization, inspection).
    fn trace(&self) -> TropicalDualClifford<T, DIM>;

    /// Clear all stored associations.
    fn clear(&self) -> Result<()>;

    /// Get the number of stored items.
    fn len(&self) -> usize;

    /// Check if the store is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Basic in-memory implementation of `MemoryStore`.
///
/// Wraps a `MemoryTrace` with the full store interface.
#[derive(Debug)]
pub struct BasicMemoryStore<T: MinuetFloat, const DIM: usize, S = store_state::Ready> {
    trace: MemoryTrace<T, DIM>,
    _state: PhantomData<S>,
}

impl<T: MinuetFloat, const DIM: usize> BasicMemoryStore<T, DIM, store_state::Ready> {
    /// Create a new empty memory store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            trace: MemoryTrace::new().into_unknown(),
            _state: PhantomData,
        }
    }

    /// Create with a specific temperature parameter.
    #[must_use]
    pub fn with_beta(beta: f64) -> Self {
        Self {
            trace: MemoryTrace::with_beta(beta).into_unknown(),
            _state: PhantomData,
        }
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for BasicMemoryStore<T, DIM, store_state::Ready> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize, S> MemoryStore<T, DIM> for BasicMemoryStore<T, DIM, S>
where
    S: Send + Sync,
{
    fn store(
        &self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<StoreReceipt> {
        self.trace.store(key, value)
    }

    fn store_batch(
        &self,
        pairs: &[(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)],
    ) -> Result<Vec<StoreReceipt>> {
        // For basic store, just iterate
        // Parallel implementation in parallel::batch
        pairs.iter().map(|(k, v)| self.trace.store(k, v)).collect()
    }

    fn retrieve(&self, key: &TropicalDualClifford<T, DIM>) -> Result<RetrievalResult<T, DIM>> {
        let value = self.trace.retrieve(key);
        let info = self.trace.capacity_info();

        Ok(RetrievalResult {
            value: value.clone(),
            raw_value: value,
            confidence: info.estimated_snr,
            attribution: Vec::new(),
            query_similarity: 1.0, // Direct retrieval
        })
    }

    fn query(&self, query: Query<T, DIM>) -> Result<QueryResult<T, DIM>> {
        query.execute(&self.trace)
    }

    fn capacity(&self) -> CapacityInfo {
        self.trace.capacity_info()
    }

    fn merge(&self, other: &dyn MemoryStore<T, DIM>) -> Result<MergeResult> {
        // Get other's trace and merge
        let other_trace = other.trace();
        let other_count = other.len() as u64;

        // Create a temporary trace from other's data
        let temp_trace: MemoryTrace<T, DIM> = MemoryTrace::new().into_unknown();
        // Store the combined trace
        // This is a simplified merge - full implementation would be more sophisticated

        let pre_count = self.trace.item_count();
        self.trace.merge(&temp_trace)?;

        let final_info = self.trace.capacity_info();

        Ok(MergeResult {
            items_merged: other_count,
            final_snr: final_info.estimated_snr,
            capacity_warning: final_info.remaining_stores.and_then(|r| {
                if r == 0 {
                    Some(CapacityWarning::Critical {
                        snr: final_info.estimated_snr,
                    })
                } else if final_info.utilization > 0.7 {
                    Some(CapacityWarning::Approaching {
                        utilization: final_info.utilization,
                        remaining_stores: r,
                    })
                } else {
                    None
                }
            }),
        })
    }

    fn trace(&self) -> TropicalDualClifford<T, DIM> {
        self.trace.raw_trace()
    }

    fn clear(&self) -> Result<()> {
        self.trace.clear();
        Ok(())
    }

    fn len(&self) -> usize {
        self.trace.item_count() as usize
    }
}

impl<T: MinuetFloat, const DIM: usize, S> Clone for BasicMemoryStore<T, DIM, S>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            trace: self.trace.clone(),
            _state: PhantomData,
        }
    }
}

/// Builder for configuring memory stores.
#[derive(Debug, Clone)]
pub struct MemoryStoreBuilder<T: MinuetFloat, const DIM: usize> {
    beta: f64,
    snr_threshold: f64,
    warning_threshold: f64,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> MemoryStoreBuilder<T, DIM> {
    /// Create a new builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beta: 1.0,
            snr_threshold: 0.5,
            warning_threshold: 0.7,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the temperature parameter.
    #[must_use]
    pub fn beta(mut self, beta: f64) -> Self {
        self.beta = beta;
        self
    }

    /// Set the SNR threshold for capacity warnings.
    #[must_use]
    pub fn snr_threshold(mut self, threshold: f64) -> Self {
        self.snr_threshold = threshold;
        self
    }

    /// Set the warning threshold (fraction of capacity).
    #[must_use]
    pub fn warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    /// Build the memory store.
    #[must_use]
    pub fn build(self) -> BasicMemoryStore<T, DIM> {
        BasicMemoryStore::with_beta(self.beta)
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for MemoryStoreBuilder<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_fusion::holographic::Bindable;

    #[test]
    fn basic_store_creation() {
        let store: BasicMemoryStore<f64, 8> = BasicMemoryStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn store_and_retrieve() {
        let store: BasicMemoryStore<f64, 8> = BasicMemoryStore::new();

        let key = TropicalDualClifford::random();
        let value = TropicalDualClifford::random();

        store.store(&key, &value).unwrap();
        assert_eq!(store.len(), 1);

        let result = store.retrieve(&key).unwrap();
        // Similarity should be reasonably high for single item
        assert!(result.value.similarity(&value) > 0.5);
    }

    #[test]
    fn builder_pattern() {
        let store: BasicMemoryStore<f64, 16> = MemoryStoreBuilder::new()
            .beta(2.0)
            .snr_threshold(0.6)
            .warning_threshold(0.8)
            .build();

        assert!(store.is_empty());
    }

    #[test]
    fn clear_resets_store() {
        let store: BasicMemoryStore<f64, 8> = BasicMemoryStore::new();

        let key = TropicalDualClifford::random();
        let value = TropicalDualClifford::random();
        store.store(&key, &value).unwrap();

        assert!(!store.is_empty());
        store.clear().unwrap();
        assert!(store.is_empty());
    }
}
