// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Symbol codebook implementation.
//!
//! Provides consistent symbol-to-vector mapping.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use amari_holographic::BindingAlgebra;
use parking_lot::RwLock;

use crate::error::MinuetResult;
use crate::traits::Codebook;

/// A hash-map backed symbol codebook.
///
/// Provides consistent, deterministic mapping from string symbols to
/// algebraic representations. Symbols are generated on first access
/// and cached for subsequent lookups.
///
/// # Example
///
/// ```rust
/// # use minuet::prelude::*;
/// # type Algebra = ProductCliffordAlgebra<64>;
/// let codebook = HashMapCodebook::<Algebra>::new();
///
/// let paris = codebook.symbol("paris");
/// let france = codebook.symbol("france");
///
/// // Same name always returns same representation
/// let paris2 = codebook.symbol("paris");
/// assert!(paris.similarity(&paris2) > 0.99);
/// ```
pub struct HashMapCodebook<A: BindingAlgebra> {
    symbols: RwLock<HashMap<String, A>>,
}

impl<A: BindingAlgebra> Default for HashMapCodebook<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple linear congruential generator for deterministic randomness.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        // LCG constants (same as glibc)
        self.state = self.state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        // Map to [-1, 1] range
        ((self.state as f64) / (u64::MAX as f64)) * 2.0 - 1.0
    }
}

impl<A: BindingAlgebra> HashMapCodebook<A> {
    /// Create a new empty codebook.
    #[must_use]
    pub fn new() -> Self {
        Self {
            symbols: RwLock::new(HashMap::new()),
        }
    }

    /// Create with pre-registered symbols.
    #[must_use]
    pub fn with_symbols(symbols: impl IntoIterator<Item = (String, A)>) -> Self {
        Self {
            symbols: RwLock::new(symbols.into_iter().collect()),
        }
    }

    /// Generate a deterministic symbol representation from a name.
    ///
    /// Uses a hash of the name to seed a pseudo-random number generator,
    /// then generates coefficients for the algebra element.
    fn generate_symbol_from_name(name: &str) -> A {
        // Hash the name to get a seed
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        let seed = hasher.finish();

        // Determine dimension from identity element
        let dim = A::identity().dimension();

        // Generate pseudo-random coefficients
        let mut rng = SimpleRng::new(seed);
        let coeffs: Vec<f64> = (0..dim).map(|_| rng.next_f64()).collect();

        // Create and normalize the element
        A::from_coefficients(&coeffs)
            .and_then(|elem| elem.normalize())
            .unwrap_or_else(|_| A::identity())
    }
}

impl<A: BindingAlgebra> Codebook for HashMapCodebook<A> {
    type Algebra = A;

    fn symbol(&self, name: &str) -> A {
        // Fast path: check if exists
        {
            let symbols = self.symbols.read();
            if let Some(repr) = symbols.get(name) {
                return repr.clone();
            }
        }

        // Slow path: generate and insert
        let mut symbols = self.symbols.write();
        // Double-check in case another thread inserted
        if let Some(repr) = symbols.get(name) {
            return repr.clone();
        }

        let repr = Self::generate_symbol_from_name(name);
        symbols.insert(name.to_string(), repr.clone());
        repr
    }

    fn get(&self, name: &str) -> Option<A> {
        self.symbols.read().get(name).cloned()
    }

    fn register(&self, name: &str, repr: A) -> MinuetResult<()> {
        self.symbols.write().insert(name.to_string(), repr);
        Ok(())
    }

    fn len(&self) -> usize {
        self.symbols.read().len()
    }

    fn all_symbols(&self) -> Vec<A> {
        self.symbols.read().values().cloned().collect()
    }

    fn all_names(&self) -> Vec<String> {
        self.symbols.read().keys().cloned().collect()
    }

    fn closest(&self, repr: &A) -> Option<(String, f64)> {
        let symbols = self.symbols.read();
        symbols
            .iter()
            .map(|(name, sym)| (name.clone(), repr.similarity(sym)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>; // 64 dimensions

    #[test]
    fn symbol_consistency() {
        let codebook = HashMapCodebook::<TestAlgebra>::new();

        let a1 = codebook.symbol("test");
        let a2 = codebook.symbol("test");

        // Same name should give same representation
        assert!(a1.similarity(&a2) > 0.99);
    }

    #[test]
    fn different_symbols_dissimilar() {
        let codebook = HashMapCodebook::<TestAlgebra>::new();

        let a = codebook.symbol("foo");
        let b = codebook.symbol("bar");

        // Different names should give dissimilar representations
        let sim = a.similarity(&b).abs();
        assert!(sim < 0.5, "similarity was {}", sim);
    }

    #[test]
    fn closest_finds_match() {
        let codebook = HashMapCodebook::<TestAlgebra>::new();

        let _ = codebook.symbol("alpha");
        let _ = codebook.symbol("beta");
        let target = codebook.symbol("gamma");

        let (name, sim) = codebook.closest(&target).unwrap();
        assert_eq!(name, "gamma");
        assert!(sim > 0.99);
    }

    #[test]
    fn register_custom_symbol() {
        let codebook = HashMapCodebook::<TestAlgebra>::new();

        // Use the codebook to generate a custom symbol deterministically
        let custom = HashMapCodebook::<TestAlgebra>::generate_symbol_from_name("custom_source");
        codebook.register("custom", custom.clone()).unwrap();

        let retrieved = codebook.symbol("custom");
        assert!(retrieved.similarity(&custom) > 0.99);
    }
}
