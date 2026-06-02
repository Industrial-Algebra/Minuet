// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Dense trace implementation.
//!
//! The fundamental holographic storage unit.

use std::sync::atomic::{AtomicU64, Ordering};

use amari_holographic::BindingAlgebra;
use parking_lot::RwLock;

use crate::traits::MemoryTrace;

/// A dense holographic memory trace.
///
/// Stores items in superposition using the bundling operation.
/// This is the fundamental building block for all memory stores.
///
/// # Example
///
/// ```rust
/// # use minuet::prelude::*;
/// # type Algebra = ProductCliffordAlgebra<32>;
/// # fn main() -> MinuetResult<()> {
/// let mut trace = DenseTrace::<Algebra>::new();
///
/// let item = Algebra::random_versor(2);
/// trace.add(&item, 1.0);
///
/// assert!(trace.similarity(&item) > 0.5);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct DenseTrace<A: BindingAlgebra> {
    /// The superposed trace.
    trace: RwLock<A>,
    /// Number of items added.
    item_count: AtomicU64,
    /// Temperature for bundling (default: 1.0 = soft).
    beta: f64,
}

impl<A: BindingAlgebra> Clone for DenseTrace<A> {
    fn clone(&self) -> Self {
        Self {
            trace: RwLock::new(self.trace.read().clone()),
            item_count: AtomicU64::new(self.item_count.load(Ordering::Relaxed)),
            beta: self.beta,
        }
    }
}

impl<A: BindingAlgebra> Default for DenseTrace<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> DenseTrace<A> {
    /// Create a new empty trace.
    #[must_use]
    pub fn new() -> Self {
        Self {
            trace: RwLock::new(A::zero()),
            item_count: AtomicU64::new(0),
            beta: 1.0,
        }
    }

    /// Create with custom temperature.
    #[must_use]
    pub fn with_temperature(beta: f64) -> Self {
        Self {
            trace: RwLock::new(A::zero()),
            item_count: AtomicU64::new(0),
            beta,
        }
    }

    /// Get the bundling temperature.
    #[must_use]
    pub fn temperature(&self) -> f64 {
        self.beta
    }
}

/// Scale an algebra element by a weight factor.
///
/// Multiplies all coefficients by the weight.
fn scale_element<A: BindingAlgebra>(elem: &A, weight: f64) -> A {
    let coeffs: Vec<f64> = elem
        .to_coefficients()
        .into_iter()
        .map(|c| c * weight)
        .collect();
    A::from_coefficients(&coeffs).unwrap_or_else(|_| elem.clone())
}

impl<A: BindingAlgebra> MemoryTrace for DenseTrace<A> {
    type Algebra = A;

    fn dimension(&self) -> usize {
        // Call dimension() on the zero element
        A::zero().dimension()
    }

    fn item_count(&self) -> usize {
        self.item_count.load(Ordering::Relaxed) as usize
    }

    fn add(&mut self, item: &A, weight: f64) {
        let mut trace = self.trace.write();
        let scaled = scale_element(item, weight);
        *trace = trace
            .bundle(&scaled, self.beta)
            .unwrap_or_else(|_| trace.clone());
        self.item_count.fetch_add(1, Ordering::Relaxed);
    }

    fn merge(&mut self, other: &Self, weight: f64) {
        let mut trace = self.trace.write();
        let other_trace = other.trace.read();
        let scaled = scale_element(&*other_trace, weight);
        *trace = trace
            .bundle(&scaled, self.beta)
            .unwrap_or_else(|_| trace.clone());
        self.item_count
            .fetch_add(other.item_count.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    fn clear(&mut self) {
        *self.trace.write() = A::zero();
        self.item_count.store(0, Ordering::Relaxed);
    }

    fn similarity(&self, query: &A) -> f64 {
        self.trace.read().similarity(query)
    }

    fn unbind(&self, query: &A) -> A {
        let trace = self.trace.read();
        // unbind(query, trace) = query^-1 * trace
        query
            .inverse()
            .map_or_else(|_| trace.clone(), |inv| inv.bind(&*trace))
    }

    fn as_algebra(&self) -> A {
        self.trace.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>; // 64 dimensions

    /// Generate a test element deterministically from a seed string.
    fn test_element(seed: &str) -> TestAlgebra {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let mut state = hasher.finish();

        let dim = TestAlgebra::identity().dimension();
        let coeffs: Vec<f64> = (0..dim)
            .map(|_| {
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
                ((state as f64) / (u64::MAX as f64)) * 2.0 - 1.0
            })
            .collect();

        TestAlgebra::from_coefficients(&coeffs)
            .and_then(|e| e.normalize())
            .unwrap()
    }

    #[test]
    fn new_trace_is_empty() {
        let trace = DenseTrace::<TestAlgebra>::new();
        assert_eq!(trace.item_count(), 0);
        assert!(trace.is_empty());
    }

    #[test]
    fn add_increases_count() {
        let mut trace = DenseTrace::<TestAlgebra>::new();
        let item = test_element("test_item_1");
        trace.add(&item, 1.0);
        assert_eq!(trace.item_count(), 1);
    }

    #[test]
    fn similar_after_add() {
        let mut trace = DenseTrace::<TestAlgebra>::new();
        let item = test_element("test_item_2");
        trace.add(&item, 1.0);
        // After adding one item, similarity should be high
        let sim = trace.similarity(&item);
        assert!(sim > 0.5, "similarity was {}", sim);
    }

    #[test]
    fn clear_resets_trace() {
        let mut trace = DenseTrace::<TestAlgebra>::new();
        let item = test_element("test_item_3");
        trace.add(&item, 1.0);
        trace.clear();
        assert_eq!(trace.item_count(), 0);
        assert!(trace.is_empty());
    }
}
