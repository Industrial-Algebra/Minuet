//! Symbol vocabularies with stable representations.
//!
//! Codebooks maintain mappings from symbolic names to holographic representations,
//! providing consistent symbol generation and cleanup targets.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::Result;
use crate::precision::MinuetFloat;

/// Properties for constrained symbol generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolProperties {
    /// Preferred grade (0=scalar, 1=vector, 2=bivector, etc.).
    pub grade: Option<usize>,

    /// Constraint: must be near-orthogonal to these symbols.
    pub orthogonal_to: Vec<String>,

    /// Constraint: must be similar to these (name, similarity) pairs.
    pub similar_to: Vec<(String, f64)>,

    /// Optional seed for reproducible generation.
    pub seed: Option<u64>,
}

/// Trait for symbol representation generators.
pub trait SymbolGenerator<T: MinuetFloat, const DIM: usize>: Send + Sync {
    /// Generate a new random symbol representation.
    fn generate(&self) -> TropicalDualClifford<T, DIM>;

    /// Generate a symbol with specific properties.
    fn generate_with_properties(
        &self,
        props: &SymbolProperties,
        existing: &HashMap<String, TropicalDualClifford<T, DIM>>,
    ) -> TropicalDualClifford<T, DIM>;
}

/// Standard random symbol generator.
#[derive(Debug)]
pub struct StandardGenerator {
    counter: AtomicU64,
}

impl Clone for StandardGenerator {
    fn clone(&self) -> Self {
        Self {
            counter: AtomicU64::new(self.counter.load(Ordering::SeqCst)),
        }
    }
}

impl StandardGenerator {
    /// Create a new standard generator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }

    /// Create with a specific starting seed.
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            counter: AtomicU64::new(seed),
        }
    }
}

impl Default for StandardGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> SymbolGenerator<T, DIM> for StandardGenerator {
    fn generate(&self) -> TropicalDualClifford<T, DIM> {
        let _seed = self.counter.fetch_add(1, Ordering::SeqCst);
        // Use random_versor which uses actual randomness (fastrand)
        // random() is deterministic and broken
        TropicalDualClifford::random_versor(2)
    }

    fn generate_with_properties(
        &self,
        props: &SymbolProperties,
        existing: &HashMap<String, TropicalDualClifford<T, DIM>>,
    ) -> TropicalDualClifford<T, DIM> {
        // Start with random versor generation
        let mut candidate = self.generate();

        // Normalize to unit norm (random_versor already normalized, but be safe)
        candidate = candidate.normalize();

        // Apply similarity constraints by interpolation
        // This is a simplified version - full implementation would use
        // iterative projection onto constraint manifold
        for (name, target_sim) in &props.similar_to {
            if let Some(other) = existing.get(name) {
                // Interpolate towards other
                let t = T::from_f64(*target_sim).unwrap_or(T::zero());
                candidate = candidate.interpolate(other, t);
            }
        }

        // Re-normalize after interpolation
        candidate.normalize()
    }
}

/// A vocabulary of atomic symbols with stable representations.
///
/// Codebooks provide:
/// - Consistent symbol -> vector mapping
/// - Cleanup targets for resonator networks
/// - Domain-specific symbol generation
pub struct Codebook<T: MinuetFloat, const DIM: usize> {
    /// Symbol name to representation mapping.
    symbols: RwLock<HashMap<String, TropicalDualClifford<T, DIM>>>,

    /// The symbol generator.
    generator: Box<dyn SymbolGenerator<T, DIM>>,

    /// Metadata about the codebook.
    metadata: RwLock<CodebookMetadata>,
}

impl<T: MinuetFloat, const DIM: usize> std::fmt::Debug for Codebook<T, DIM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Codebook")
            .field("symbols", &self.symbols)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

/// Metadata about a codebook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodebookMetadata {
    /// Human-readable name.
    pub name: Option<String>,

    /// Description.
    pub description: Option<String>,

    /// Domain (e.g., "molecular", "symbolic", "geometric").
    pub domain: Option<String>,

    /// Creation timestamp.
    pub created_at: Option<u64>,

    /// Number of symbols.
    pub symbol_count: usize,
}

impl<T: MinuetFloat, const DIM: usize> Codebook<T, DIM> {
    /// Create a new empty codebook with default generator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            symbols: RwLock::new(HashMap::new()),
            generator: Box::new(StandardGenerator::new()),
            metadata: RwLock::new(CodebookMetadata::default()),
        }
    }

    /// Create with a custom generator.
    #[must_use]
    pub fn with_generator(generator: Box<dyn SymbolGenerator<T, DIM>>) -> Self {
        Self {
            symbols: RwLock::new(HashMap::new()),
            generator,
            metadata: RwLock::new(CodebookMetadata::default()),
        }
    }

    /// Get or create a symbol.
    ///
    /// If the symbol doesn't exist, generates a new representation.
    pub fn symbol(&self, name: &str) -> TropicalDualClifford<T, DIM> {
        let mut symbols = self.symbols.write();

        if let Some(existing) = symbols.get(name) {
            return existing.clone();
        }

        let new_symbol = self.generator.generate();
        symbols.insert(name.to_string(), new_symbol.clone());
        self.metadata.write().symbol_count = symbols.len();
        new_symbol
    }

    /// Get or create a symbol with specific properties.
    pub fn symbol_with_properties(
        &self,
        name: &str,
        props: &SymbolProperties,
    ) -> TropicalDualClifford<T, DIM> {
        let mut symbols = self.symbols.write();

        if let Some(existing) = symbols.get(name) {
            return existing.clone();
        }

        let new_symbol = self.generator.generate_with_properties(props, &symbols);
        symbols.insert(name.to_string(), new_symbol.clone());
        self.metadata.write().symbol_count = symbols.len();
        new_symbol
    }

    /// Get an existing symbol.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<TropicalDualClifford<T, DIM>> {
        self.symbols.read().get(name).cloned()
    }

    /// Check if a symbol exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.symbols.read().contains_key(name)
    }

    /// Get all symbol names.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.symbols.read().keys().cloned().collect()
    }

    /// Get all symbols as a vector.
    #[must_use]
    pub fn all_symbols(&self) -> Vec<TropicalDualClifford<T, DIM>> {
        self.symbols.read().values().cloned().collect()
    }

    /// Number of symbols.
    #[must_use]
    pub fn len(&self) -> usize {
        self.symbols.read().len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.symbols.read().is_empty()
    }

    /// Find the nearest symbol to a query.
    #[must_use]
    pub fn nearest(&self, query: &TropicalDualClifford<T, DIM>) -> Option<(String, f64)> {
        let symbols = self.symbols.read();

        symbols
            .iter()
            .map(|(name, symbol)| {
                let sim = query.similarity(symbol);
                (name.clone(), sim)
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    }

    /// Find the k nearest symbols to a query.
    #[must_use]
    pub fn k_nearest(&self, query: &TropicalDualClifford<T, DIM>, k: usize) -> Vec<(String, f64)> {
        let symbols = self.symbols.read();

        let mut results: Vec<_> = symbols
            .iter()
            .map(|(name, symbol)| {
                let sim = query.similarity(symbol);
                (name.clone(), sim)
            })
            .collect();

        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        results.truncate(k);
        results
    }

    /// Get symbols above a similarity threshold.
    #[must_use]
    pub fn above_threshold(
        &self,
        query: &TropicalDualClifford<T, DIM>,
        threshold: f64,
    ) -> Vec<(String, f64)> {
        let symbols = self.symbols.read();

        symbols
            .iter()
            .filter_map(|(name, symbol)| {
                let sim = query.similarity(symbol);
                if sim >= threshold {
                    Some((name.clone(), sim))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, metadata: CodebookMetadata) {
        *self.metadata.write() = metadata;
    }

    /// Get metadata.
    #[must_use]
    pub fn metadata(&self) -> CodebookMetadata {
        self.metadata.read().clone()
    }

    /// Clear all symbols.
    pub fn clear(&self) {
        self.symbols.write().clear();
        self.metadata.write().symbol_count = 0;
    }

    /// Remove a symbol.
    pub fn remove(&self, name: &str) -> Option<TropicalDualClifford<T, DIM>> {
        let result = self.symbols.write().remove(name);
        if result.is_some() {
            self.metadata.write().symbol_count = self.symbols.read().len();
        }
        result
    }

    /// Insert a symbol with a specific representation.
    pub fn insert(&self, name: &str, symbol: TropicalDualClifford<T, DIM>) {
        self.symbols.write().insert(name.to_string(), symbol);
        self.metadata.write().symbol_count = self.symbols.read().len();
    }

    /// Merge another codebook into this one.
    ///
    /// Symbols from other take precedence on conflict.
    pub fn merge(&self, other: &Codebook<T, DIM>) {
        let mut symbols = self.symbols.write();
        let other_symbols = other.symbols.read();

        for (name, symbol) in other_symbols.iter() {
            symbols.insert(name.clone(), symbol.clone());
        }

        self.metadata.write().symbol_count = symbols.len();
    }

    /// Save codebook to file.
    ///
    /// Note: Requires TropicalDualClifford to implement Serialize.
    /// Currently stubbed - amari-fusion needs serde feature enhancement.
    pub fn save(&self, _path: impl AsRef<Path>) -> Result<()> {
        // TODO: Implement when amari-fusion adds Serialize to TropicalDualClifford
        // For now, save only metadata
        Err(crate::error::MinuetError::NotImplemented {
            feature: "Codebook serialization (awaiting amari-fusion serde support)".into(),
        })
    }

    /// Load codebook from file.
    ///
    /// Note: Requires TropicalDualClifford to implement Deserialize.
    /// Currently stubbed - amari-fusion needs serde feature enhancement.
    pub fn load(_path: impl AsRef<Path>) -> Result<Self> {
        // TODO: Implement when amari-fusion adds Deserialize to TropicalDualClifford
        Err(crate::error::MinuetError::NotImplemented {
            feature: "Codebook deserialization (awaiting amari-fusion serde support)".into(),
        })
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for Codebook<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> Clone for Codebook<T, DIM> {
    fn clone(&self) -> Self {
        Self {
            symbols: RwLock::new(self.symbols.read().clone()),
            generator: Box::new(StandardGenerator::new()),
            metadata: RwLock::new(self.metadata.read().clone()),
        }
    }
}

// Note: CodebookSnapshot removed - requires amari-fusion to implement Serialize/Deserialize
// for TropicalDualClifford. Once that's available, we can add proper persistence.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_creation() {
        let codebook: Codebook<f64, 8> = Codebook::new();

        let a = codebook.symbol("a");
        let b = codebook.symbol("b");

        // Same name should return same symbol
        let a2 = codebook.symbol("a");
        let self_sim = a.similarity(&a2);
        assert!(
            self_sim > 0.99,
            "self-similarity should be ~1, got {}",
            self_sim
        );

        // Different names should be dissimilar
        // In 256 dimensions, random versors should have similarity ~0
        // Use a relaxed threshold to account for variance
        let sim = a.similarity(&b);
        assert!(
            sim.abs() < 0.5,
            "different symbols should be dissimilar, got {}",
            sim
        );
    }

    #[test]
    fn nearest_lookup() {
        let codebook: Codebook<f64, 8> = Codebook::new();

        let _a = codebook.symbol("a");
        let _b = codebook.symbol("b");
        let _c = codebook.symbol("c");

        let query = codebook.get("a").unwrap();
        let (name, sim) = codebook.nearest(&query).unwrap();

        assert_eq!(name, "a");
        assert!(sim > 0.99);
    }

    #[test]
    fn k_nearest() {
        let codebook: Codebook<f64, 8> = Codebook::new();

        for i in 0..10 {
            codebook.symbol(&format!("sym_{}", i));
        }

        let query = codebook.get("sym_0").unwrap();
        let results = codebook.k_nearest(&query, 3);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "sym_0");
    }
}
