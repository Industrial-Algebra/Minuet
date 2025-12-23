//! Simple single-trace memory store.
//!
//! Minimal implementation for learning and simple use cases.

use std::sync::atomic::{AtomicU64, Ordering};

use amari_holographic::BindingAlgebra;
use parking_lot::RwLock;

use crate::error::MinuetResult;
use crate::traits::{
    CapacityInfo, CapacityWarning, MemoryStore, MemoryTrace, RetrievalResult, StoreOptions,
    StoreReceipt, TraceCapacityInfo,
};

use super::DenseTrace;

/// A simple single-trace holographic memory store.
///
/// This is the minimal useful implementation — good for learning,
/// prototyping, and small-scale use. For production, consider
/// [`ShardedStore`](super::ShardedStore).
///
/// # Example
///
/// ```rust,ignore
/// use minuet::store::SimpleStore;
/// use amari_holographic::ProductCliffordAlgebra;
///
/// type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions
///
/// let store = SimpleStore::<Algebra>::new();
///
/// let key = Algebra::random_versor(2);
/// let value = Algebra::random_versor(2);
///
/// store.store(&key, &value)?;
/// let result = store.retrieve(&key)?;
/// ```
pub struct SimpleStore<A: BindingAlgebra> {
    trace: RwLock<DenseTrace<A>>,
    next_id: AtomicU64,
    config: SimpleStoreConfig,
}

/// Configuration for SimpleStore.
#[derive(Clone, Debug)]
pub struct SimpleStoreConfig {
    /// Temperature for bundling operations.
    pub bundle_temperature: f64,
    /// Warning threshold (utilization fraction).
    pub warning_threshold: f64,
    /// Critical threshold (utilization fraction).
    pub critical_threshold: f64,
}

impl Default for SimpleStoreConfig {
    fn default() -> Self {
        Self {
            bundle_temperature: 1.0,
            warning_threshold: 0.8,
            critical_threshold: 0.95,
        }
    }
}

impl<A: BindingAlgebra> Default for SimpleStore<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> SimpleStore<A> {
    /// Create a new simple store.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(SimpleStoreConfig::default())
    }

    /// Create with configuration.
    #[must_use]
    pub fn with_config(config: SimpleStoreConfig) -> Self {
        Self {
            trace: RwLock::new(DenseTrace::with_temperature(config.bundle_temperature)),
            next_id: AtomicU64::new(0),
            config,
        }
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &SimpleStoreConfig {
        &self.config
    }
}

impl<A: BindingAlgebra> MemoryStore for SimpleStore<A> {
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
        // Bind key with value
        let bound = key.bind(value);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Add to trace
        let mut trace = self.trace.write();
        trace.add(&bound, options.weight);

        // Compute capacity info
        let snr = trace.estimated_snr();
        let utilization = trace.utilization();

        let warning = if utilization > self.config.critical_threshold {
            Some(CapacityWarning::AtCapacity)
        } else if utilization > self.config.warning_threshold {
            Some(CapacityWarning::ApproachingCapacity { utilization })
        } else {
            None
        };

        Ok(StoreReceipt {
            id,
            post_snr: snr,
            warning,
            location: "main".into(),
        })
    }

    fn store_batch(&self, pairs: &[(A, A)]) -> MinuetResult<Vec<StoreReceipt>> {
        pairs.iter().map(|(k, v)| self.store(k, v)).collect()
    }

    fn retrieve(&self, key: &A) -> MinuetResult<RetrievalResult<A>> {
        let trace = self.trace.read();
        let raw = trace.unbind(key);
        let confidence = (trace.estimated_snr() / 10.0).min(1.0);

        Ok(RetrievalResult {
            value: raw,
            confidence,
            attribution: vec![],
        })
    }

    fn capacity_info(&self) -> CapacityInfo {
        let trace = self.trace.read();
        let items = trace.item_count();
        let capacity = trace.theoretical_capacity();
        let utilization = trace.utilization();
        let snr = trace.estimated_snr();

        CapacityInfo {
            total_items: items,
            theoretical_capacity: capacity,
            utilization,
            estimated_snr: snr,
            per_trace: vec![TraceCapacityInfo {
                name: "main".into(),
                items,
                capacity,
                utilization,
            }],
        }
    }

    fn clear(&self) -> MinuetResult<()> {
        self.trace.write().clear();
        Ok(())
    }

    fn trace_count(&self) -> usize {
        1
    }

    fn total_items(&self) -> usize {
        self.trace.read().item_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>; // 64 dimensions

    #[test]
    fn store_and_retrieve() {
        let store = SimpleStore::<TestAlgebra>::new();

        let key = TestAlgebra::random_versor(2);
        let value = TestAlgebra::random_versor(2);

        let receipt = store.store(&key, &value).unwrap();
        assert_eq!(receipt.id, 0);

        let result = store.retrieve(&key).unwrap();
        // Retrieved value should be similar to original value
        let sim = result.value.similarity(&value);
        assert!(sim > 0.5, "similarity was {}", sim);
    }

    #[test]
    fn capacity_info() {
        let store = SimpleStore::<TestAlgebra>::new();

        let info = store.capacity_info();
        assert_eq!(info.total_items, 0);
        assert!(info.theoretical_capacity > 0);

        // Add some items
        for _ in 0..5 {
            let key = TestAlgebra::random_versor(2);
            let value = TestAlgebra::random_versor(2);
            store.store(&key, &value).unwrap();
        }

        let info = store.capacity_info();
        assert_eq!(info.total_items, 5);
    }
}
