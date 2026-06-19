// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Simple memory - a complete, minimal holographic memory.
//!
//! This is the easiest way to get started with Minuet.

use amari_holographic::BindingAlgebra;

use crate::encoding::HashMapCodebook;
use crate::error::MinuetResult;
use crate::store::SimpleStore;
use crate::traits::{Codebook, MemoryStore, RetrievalResult, StoreReceipt};

/// A simple holographic memory combining store and codebook.
///
/// This is the minimal complete implementation - good for learning,
/// prototyping, and small-scale use.
///
/// # Example
///
/// ```rust
/// # use minuet::prelude::*;
/// # use minuet::reference::SimpleMemory;
/// # type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions
/// # fn main() -> MinuetResult<()> {
/// let memory = SimpleMemory::<Algebra>::new();
///
/// // Store a relationship
/// let paris = memory.symbol("paris");
/// let france = memory.symbol("france");
/// memory.store(&paris, &france)?;
///
/// // Retrieve it
/// let result = memory.retrieve(&paris)?;
/// let (name, _) = memory.closest(&result.value).unwrap();
/// assert_eq!(name, "france");
/// # Ok(())
/// # }
/// ```
pub struct SimpleMemory<A: BindingAlgebra> {
    store: SimpleStore<A>,
    codebook: HashMapCodebook<A>,
}

impl<A: BindingAlgebra> Default for SimpleMemory<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> SimpleMemory<A> {
    /// Create a new simple memory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: SimpleStore::new(),
            codebook: HashMapCodebook::new(),
        }
    }

    /// Get or create a symbol representation.
    pub fn symbol(&self, name: &str) -> A {
        self.codebook.symbol(name)
    }

    /// Store a key-value association.
    pub fn store(&self, key: &A, value: &A) -> MinuetResult<StoreReceipt> {
        self.store.store(key, value)
    }

    /// Store a relationship between named symbols.
    pub fn store_symbols(&self, key: &str, value: &str) -> MinuetResult<StoreReceipt> {
        let k = self.symbol(key);
        let v = self.symbol(value);
        self.store(&k, &v)
    }

    /// Retrieve value associated with key.
    pub fn retrieve(&self, key: &A) -> MinuetResult<RetrievalResult<A>> {
        self.store.retrieve(key)
    }

    /// Retrieve using a named symbol as key.
    pub fn retrieve_symbol(&self, key: &str) -> MinuetResult<RetrievalResult<A>> {
        let k = self.symbol(key);
        self.retrieve(&k)
    }

    /// Find closest symbol to a representation.
    pub fn closest(&self, repr: &A) -> Option<(String, f64)> {
        self.codebook.closest(repr)
    }

    /// Recall: retrieve and find closest symbol.
    ///
    /// Returns (symbol_name, confidence) if found.
    pub fn recall(&self, key: &str) -> MinuetResult<Option<(String, f64)>> {
        let result = self.retrieve_symbol(key)?;
        Ok(self.closest(&result.value))
    }

    /// Get the codebook.
    #[must_use]
    pub fn codebook(&self) -> &HashMapCodebook<A> {
        &self.codebook
    }

    /// Get capacity info.
    #[must_use]
    pub fn capacity_info(&self) -> crate::traits::CapacityInfo {
        self.store.capacity_info()
    }

    /// Clear all stored associations.
    pub fn clear(&self) -> MinuetResult<()> {
        self.store.clear()
    }

    /// Number of symbols in codebook.
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.codebook.len()
    }

    /// Number of items stored.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.store.total_items()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<16>; // 128 dimensions

    #[test]
    fn store_and_recall() {
        let memory = SimpleMemory::<TestAlgebra>::new();

        memory.store_symbols("paris", "france").unwrap();
        memory.store_symbols("berlin", "germany").unwrap();

        let result = memory.recall("paris").unwrap();
        assert!(result.is_some());
        let (_name, sim) = result.unwrap();
        // Should retrieve with high similarity
        assert!(sim > 0.3, "similarity was {}", sim);
    }

    #[test]
    fn symbol_consistency() {
        let memory = SimpleMemory::<TestAlgebra>::new();

        let a1 = memory.symbol("test");
        let a2 = memory.symbol("test");

        assert!(a1.similarity(&a2) > 0.99);
    }

    #[test]
    fn capacity_info() {
        let memory = SimpleMemory::<TestAlgebra>::new();

        assert_eq!(memory.item_count(), 0);
        assert_eq!(memory.symbol_count(), 0);

        memory.store_symbols("a", "b").unwrap();

        assert_eq!(memory.item_count(), 1);
        assert_eq!(memory.symbol_count(), 2); // "a" and "b"
    }
}
