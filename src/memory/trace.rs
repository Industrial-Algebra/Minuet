//! Core holographic trace representation.
//!
//! The trace is the fundamental data structure for holographic memory,
//! representing the superposition of all stored key-value bindings.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

use amari_fusion::holographic::{Bindable, TropicalDualClifford};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{MinuetError, Result};
use crate::precision::MinuetFloat;

use super::capacity::{CapacityInfo, CapacityTracker};
use super::store::StoreReceipt;

/// Phantom type marker for trace state.
pub mod state {
    /// Trace is empty (no items stored).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Empty;

    /// Trace contains at least one item.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct NonEmpty;

    /// Trace state is unknown (e.g., after deserialization).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Unknown;
}

/// A holographic memory trace.
///
/// The trace is a single TDC element representing the superposition of all
/// stored key-value bindings. Storage adds bindings via bundling (soft max),
/// and retrieval unbinds to recover values.
///
/// # Type Parameters
///
/// * `T` - Numeric type (f32, f64)
/// * `DIM` - Representation dimensionality (const generic)
/// * `S` - Trace state marker (Empty, NonEmpty, Unknown)
///
/// # Invariants
///
/// - The trace is always a valid TDC element
/// - Store operations monotonically increase item count
/// - SNR decreases monotonically as items are added
#[derive(Debug)]
pub struct MemoryTrace<T, const DIM: usize, S = state::Unknown> {
    /// The superposed holographic trace.
    trace: RwLock<TropicalDualClifford<T, DIM>>,

    /// Capacity tracker for SNR estimation.
    capacity: CapacityTracker<T>,

    /// Monotonically increasing operation counter.
    operation_counter: AtomicU64,

    /// Number of items stored.
    item_count: AtomicU64,

    /// Temperature parameter for bundling.
    beta: T,

    /// Phantom marker for state.
    _state: PhantomData<S>,
}

impl<T: MinuetFloat, const DIM: usize> MemoryTrace<T, DIM, state::Empty> {
    /// Create a new empty trace.
    ///
    /// # Returns
    ///
    /// A trace in the Empty state with bundling identity as the initial value.
    #[cfg_attr(feature = "contracts", ensures(result.item_count() == 0))]
    #[must_use]
    pub fn new() -> Self {
        Self::with_beta(T::from_f64(1.0).unwrap())
    }

    /// Create a new empty trace with specified temperature.
    ///
    /// # Arguments
    ///
    /// * `beta` - Temperature parameter (higher = sharper bundling)
    #[cfg_attr(feature = "contracts", requires(beta > T::zero()))]
    #[cfg_attr(feature = "contracts", ensures(result.item_count() == 0))]
    #[must_use]
    pub fn with_beta(beta: T) -> Self {
        Self {
            trace: RwLock::new(TropicalDualClifford::bundling_zero()),
            capacity: CapacityTracker::new(DIM),
            operation_counter: AtomicU64::new(0),
            item_count: AtomicU64::new(0),
            beta,
            _state: PhantomData,
        }
    }
}

impl<T: MinuetFloat, const DIM: usize, S> MemoryTrace<T, DIM, S> {
    /// Store a key-value binding in the trace.
    ///
    /// The binding is computed as `key ⊛ value` and bundled into the trace.
    ///
    /// # Arguments
    ///
    /// * `key` - The key (query pattern)
    /// * `value` - The value to associate with the key
    ///
    /// # Returns
    ///
    /// A store receipt containing the operation ID and updated SNR estimate.
    ///
    /// # Errors
    ///
    /// Returns `MinuetError::AtCapacity` if SNR has dropped below threshold.
    #[cfg_attr(feature = "contracts", ensures(result.is_ok() ==> self.item_count() == old(self.item_count()) + 1))]
    pub fn store(
        &self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<StoreReceipt> {
        // Check capacity before storing
        let current_snr = self.capacity.estimate_snr(self.item_count() + 1);
        let threshold = self.capacity.snr_threshold();

        if current_snr < threshold {
            return Err(MinuetError::AtCapacity {
                snr: current_snr.to_f64().unwrap(),
                threshold: threshold.to_f64().unwrap(),
            });
        }

        // Compute binding: key ⊛ value
        let binding = key.bind(value);

        // Bundle into trace
        {
            let mut trace = self.trace.write();
            *trace = trace.bundle(&binding, self.beta);
        }

        // Update counters
        let op_id = self.operation_counter.fetch_add(1, Ordering::SeqCst);
        self.item_count.fetch_add(1, Ordering::SeqCst);

        // Update capacity tracker
        self.capacity.record_store();

        Ok(StoreReceipt {
            id: op_id,
            post_store_snr: current_snr.to_f64().unwrap(),
            capacity_warning: self.capacity.check_warning(),
        })
    }

    /// Retrieve the value associated with a key.
    ///
    /// Unbinds the key from the trace to recover the superposed value.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to retrieve
    ///
    /// # Returns
    ///
    /// The retrieved TDC element (may need cleanup for best results).
    #[cfg_attr(feature = "contracts", requires(self.item_count() > 0))]
    pub fn retrieve(&self, key: &TropicalDualClifford<T, DIM>) -> TropicalDualClifford<T, DIM> {
        let trace = self.trace.read();
        key.unbind(&trace)
    }

    /// Get the current item count.
    #[must_use]
    pub fn item_count(&self) -> u64 {
        self.item_count.load(Ordering::SeqCst)
    }

    /// Get capacity information.
    #[must_use]
    pub fn capacity_info(&self) -> CapacityInfo {
        self.capacity.info(self.item_count())
    }

    /// Get the raw trace for inspection or serialization.
    #[must_use]
    pub fn raw_trace(&self) -> TropicalDualClifford<T, DIM> {
        self.trace.read().clone()
    }

    /// Get the temperature parameter.
    #[must_use]
    pub fn beta(&self) -> T {
        self.beta
    }

    /// Clear all stored items, resetting to empty state.
    pub fn clear(&self) {
        let mut trace = self.trace.write();
        *trace = TropicalDualClifford::bundling_zero();
        self.item_count.store(0, Ordering::SeqCst);
        self.capacity.reset();
    }

    /// Merge another trace into this one.
    ///
    /// The traces are bundled together. Both traces should use the same beta.
    ///
    /// # Arguments
    ///
    /// * `other` - The trace to merge in
    ///
    /// # Errors
    ///
    /// Returns `MinuetError::MergeFailed` if the merge would exceed capacity.
    pub fn merge(&self, other: &Self) -> Result<()> {
        let combined_count = self.item_count() + other.item_count();
        let projected_snr = self.capacity.estimate_snr(combined_count);

        if projected_snr < self.capacity.snr_threshold() {
            return Err(MinuetError::MergeFailed(format!(
                "Merge would exceed capacity: projected SNR {:.3} < threshold {:.3}",
                projected_snr.to_f64().unwrap(),
                self.capacity.snr_threshold().to_f64().unwrap()
            )));
        }

        let other_trace = other.trace.read();
        {
            let mut trace = self.trace.write();
            *trace = trace.bundle(&other_trace, self.beta);
        }

        self.item_count
            .fetch_add(other.item_count(), Ordering::SeqCst);

        Ok(())
    }

    /// Create a snapshot for serialization.
    pub fn snapshot(&self) -> TraceSnapshot<T, DIM> {
        TraceSnapshot {
            trace: self.raw_trace(),
            item_count: self.item_count(),
            beta: self.beta,
            operation_counter: self.operation_counter.load(Ordering::SeqCst),
        }
    }

    /// Restore from a snapshot.
    pub fn from_snapshot(snapshot: TraceSnapshot<T, DIM>) -> MemoryTrace<T, DIM, state::Unknown> {
        MemoryTrace {
            trace: RwLock::new(snapshot.trace),
            capacity: CapacityTracker::new(DIM),
            operation_counter: AtomicU64::new(snapshot.operation_counter),
            item_count: AtomicU64::new(snapshot.item_count),
            beta: snapshot.beta,
            _state: PhantomData,
        }
    }

    /// Convert to unknown state (for type erasure).
    pub fn into_unknown(self) -> MemoryTrace<T, DIM, state::Unknown> {
        MemoryTrace {
            trace: self.trace,
            capacity: self.capacity,
            operation_counter: self.operation_counter,
            item_count: self.item_count,
            beta: self.beta,
            _state: PhantomData,
        }
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for MemoryTrace<T, DIM, state::Empty> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize, S> Clone for MemoryTrace<T, DIM, S>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            trace: RwLock::new(self.trace.read().clone()),
            capacity: self.capacity.clone(),
            operation_counter: AtomicU64::new(self.operation_counter.load(Ordering::SeqCst)),
            item_count: AtomicU64::new(self.item_count.load(Ordering::SeqCst)),
            beta: self.beta,
            _state: PhantomData,
        }
    }
}

/// Serializable snapshot of a trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSnapshot<T, const DIM: usize> {
    /// The trace data.
    pub trace: TropicalDualClifford<T, DIM>,
    /// Number of items stored.
    pub item_count: u64,
    /// Temperature parameter.
    pub beta: T,
    /// Operation counter.
    pub operation_counter: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trace_creation() {
        let trace: MemoryTrace<f64, 64, state::Empty> = MemoryTrace::new();
        assert_eq!(trace.item_count(), 0);
    }

    #[test]
    fn store_increments_count() {
        let trace: MemoryTrace<f64, 64, state::Empty> = MemoryTrace::new();

        let key = TropicalDualClifford::random();
        let value = TropicalDualClifford::random();

        let receipt = trace.store(&key, &value).unwrap();
        assert_eq!(receipt.id, 0);
        assert_eq!(trace.item_count(), 1);
    }

    #[test]
    fn snapshot_roundtrip() {
        let trace: MemoryTrace<f64, 64, state::Empty> = MemoryTrace::with_beta(2.0);

        let key = TropicalDualClifford::random();
        let value = TropicalDualClifford::random();
        trace.store(&key, &value).unwrap();

        let snapshot = trace.snapshot();
        let restored = MemoryTrace::from_snapshot(snapshot);

        assert_eq!(restored.item_count(), trace.item_count());
        assert!((restored.beta() - trace.beta()).abs() < 1e-10);
    }
}
