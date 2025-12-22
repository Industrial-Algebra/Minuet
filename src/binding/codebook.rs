//! Symbol vocabularies with stable representations.
//!
//! Codebooks maintain mappings from symbolic names to holographic representations,
//! providing consistent symbol generation and cleanup targets.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use amari_fusion::holographic::TropicalDualClifford;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{MinuetError, Result};
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
#[derive(Debug, Clone)]
pub struct StandardGenerator {
    counter: AtomicU64,
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
        TropicalDualClifford::random()
    }

    fn generate_with_properties(
        &self,
        props: &SymbolProperties,
        existing: &HashMap<String, TropicalDualClifford<T, DIM>>,
    ) -> TropicalDualClifford<T, DIM> {
        // Start with random generation
        let mut candidate = self.generate();

        // Apply grade constraint if specified
        if let Some(grade) = props.grade {
            candidate = candidate.project_grade(grade);
            // Normalize after projection
            let mag = candidate.magnitude();
            if mag > T::MIN_POSITIVE {
                candidate = candidate.scale(T::one() / mag);
            }
        }

        // Apply orthogonality constraints via Gram-Schmidt-like process
        for name in &props.orthogonal_to {
            if let Some(other) = existing.get(name) {
                // Remove component along other
                let proj = candidate.inner_product(other);
                let other_sq = other.inner_product(other);
                if other_sq > T::MIN_POSITIVE {
                    let scale = proj / other_sq;
                    candidate = candidate.sub(&other.scale(scale));
                }
            }
        }

        // Renormalize after orthogonalization
        let mag = candidate.magnitude();
        if mag > T::MIN_POSITIVE {
            candidate = candidate.scale(T::one() / mag);
        }

        // Apply similarity constraints by weighted averaging
        for (name, target_sim) in &props.similar_to {
            if let Some(other) = existing.get(name) {
                // Interpolate towards other by target_sim amount
                let t = T::from_f64(*target_sim).unwrap();
                candidate = candidate.scale(T::one() - t).add(&other.scale(t));
            }
        }

        // Final normalization
        let mag = candidate.magnitude();
        if mag > T::MIN_POSITIVE {
            candidate = candidate.scale(T::one() / mag);
        }

        candidate
    }
}

/// A vocabulary of atomic symbols with stable representations.
///
/// Codebooks provide:
/// - Consistent symbol -> vector mapping
/// - Cleanup targets for resonator networks
/// - Domain-specific symbol generation
#[derive(Debug)]
pub struct Codebook<T, const DIM: usize> {
    /// Symbol name to representation mapping.
    symbols: RwLock<HashMap<String, TropicalDualClifford<T, DIM>>>,

    /// The symbol generator.
    generator: Box<dyn SymbolGenerator<T, DIM>>,

    /// Metadata about the codebook.
    metadata: RwLock<CodebookMetadata>,
}

/// Metadata about a codebook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodebookMetadata {
    /// Human-readable name.
    pub name: Option<String>,

    /// Description of the codebook's domain.
    pub description: Option<String>,

    /// Version for compatibility tracking.
    pub version: u32,

    /// Number of symbols created.
    pub symbol_count: usize,
}

impl<T: MinuetFloat, const DIM: usize> Codebook<T, DIM> {
    /// Create a new empty codebook with default generator.
    #[must_use]
    pub fn new() -> Self {
        Self::with_generator(StandardGenerator::new())
    }

    /// Create with a specific symbol generator.
    #[must_use]
    pub fn with_generator<G: SymbolGenerator<T, DIM> + 'static>(generator: G) -> Self {
        Self {
            symbols: RwLock::new(HashMap::new()),
            generator: Box::new(generator),
            metadata: RwLock::new(CodebookMetadata::default()),
        }
    }

    /// Get or create a symbol.
    ///
    /// If the symbol exists, returns its representation.
    /// Otherwise, generates a new random representation and stores it.
    #[must_use]
    pub fn symbol(&self, name: &str) -> TropicalDualClifford<T, DIM> {
        // Try read lock first
        {
            let symbols = self.symbols.read();
            if let Some(repr) = symbols.get(name) {
                return repr.clone();
            }
        }

        // Need to create - upgrade to write lock
        let mut symbols = self.symbols.write();

        // Double-check (another thread may have created it)
        if let Some(repr) = symbols.get(name) {
            return repr.clone();
        }

        // Generate new symbol
        let repr = self.generator.generate();
        symbols.insert(name.to_string(), repr.clone());

        // Update metadata
        self.metadata.write().symbol_count += 1;

        repr
    }

    /// Get or create a symbol with specific properties.
    pub fn symbol_with_properties(
        &self,
        name: &str,
        props: &SymbolProperties,
    ) -> TropicalDualClifford<T, DIM> {
        // Try read lock first
        {
            let symbols = self.symbols.read();
            if let Some(repr) = symbols.get(name) {
                return repr.clone();
            }
        }

        // Need to create - upgrade to write lock
        let mut symbols = self.symbols.write();

        // Double-check
        if let Some(repr) = symbols.get(name) {
            return repr.clone();
        }

        // Generate with properties
        let repr = self.generator.generate_with_properties(props, &symbols);
        symbols.insert(name.to_string(), repr.clone());

        self.metadata.write().symbol_count += 1;

        repr
    }

    /// Get a symbol if it exists.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<TropicalDualClifford<T, DIM>> {
        self.symbols.read().get(name).cloned()
    }

    /// Register a specific representation for a symbol.
    ///
    /// Overwrites any existing representation.
    pub fn register(&self, name: &str, repr: TropicalDualClifford<T, DIM>) {
        let mut symbols = self.symbols.write();
        let is_new = !symbols.contains_key(name);
        symbols.insert(name.to_string(), repr);

        if is_new {
            self.metadata.write().symbol_count += 1;
        }
    }

    /// Get all symbols as a vector (for resonator cleanup).
    #[must_use]
    pub fn all_symbols(&self) -> Vec<TropicalDualClifford<T, DIM>> {
        self.symbols.read().values().cloned().collect()
    }

    /// Get all symbol names.
    #[must_use]
    pub fn symbol_names(&self) -> Vec<String> {
        self.symbols.read().keys().cloned().collect()
    }

    /// Number of symbols in the codebook.
    #[must_use]
    pub fn len(&self) -> usize {
        self.symbols.read().len()
    }

    /// Check if the codebook is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.symbols.read().is_empty()
    }

    /// Remove a symbol.
    pub fn remove(&self, name: &str) -> Option<TropicalDualClifford<T, DIM>> {
        let result = self.symbols.write().remove(name);
        if result.is_some() {
            self.metadata.write().symbol_count -= 1;
        }
        result
    }

    /// Clear all symbols.
    pub fn clear(&self) {
        self.symbols.write().clear();
        self.metadata.write().symbol_count = 0;
    }

    /// Set codebook metadata.
    pub fn set_metadata(&self, name: Option<String>, description: Option<String>) {
        let mut meta = self.metadata.write();
        meta.name = name;
        meta.description = description;
    }

    /// Get codebook metadata.
    #[must_use]
    pub fn metadata(&self) -> CodebookMetadata {
        self.metadata.read().clone()
    }

    /// Compute pairwise similarities between all symbols.
    #[must_use]
    pub fn similarity_matrix(&self) -> Vec<Vec<f64>> {
        let symbols = self.all_symbols();
        let n = symbols.len();

        let mut matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in 0..n {
                matrix[i][j] = symbols[i].similarity(&symbols[j]).to_f64().unwrap_or(0.0);
            }
        }

        matrix
    }

    /// Find the most similar symbol to a query.
    #[must_use]
    pub fn nearest(&self, query: &TropicalDualClifford<T, DIM>) -> Option<(String, f64)> {
        let symbols = self.symbols.read();

        let mut best: Option<(String, f64)> = None;

        for (name, repr) in symbols.iter() {
            let sim = query.similarity(repr).to_f64().unwrap_or(0.0);
            match &best {
                None => best = Some((name.clone(), sim)),
                Some((_, best_sim)) if sim > *best_sim => {
                    best = Some((name.clone(), sim));
                }
                _ => {}
            }
        }

        best
    }

    /// Find top-k most similar symbols.
    #[must_use]
    pub fn nearest_k(&self, query: &TropicalDualClifford<T, DIM>, k: usize) -> Vec<(String, f64)> {
        let symbols = self.symbols.read();

        let mut results: Vec<(String, f64)> = symbols
            .iter()
            .map(|(name, repr)| {
                let sim = query.similarity(repr).to_f64().unwrap_or(0.0);
                (name.clone(), sim)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(k);

        results
    }

    /// Save the codebook to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>
    where
        T: Serialize,
    {
        let snapshot = self.snapshot();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &snapshot)?;
        Ok(())
    }

    /// Load a codebook from a file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self>
    where
        T: for<'de> Deserialize<'de>,
    {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let snapshot: CodebookSnapshot<T, DIM> = bincode::deserialize_from(reader)?;
        Ok(Self::from_snapshot(snapshot))
    }

    /// Create a serializable snapshot.
    fn snapshot(&self) -> CodebookSnapshot<T, DIM>
    where
        T: Clone,
    {
        CodebookSnapshot {
            symbols: self.symbols.read().clone(),
            metadata: self.metadata.read().clone(),
        }
    }

    /// Restore from a snapshot.
    fn from_snapshot(snapshot: CodebookSnapshot<T, DIM>) -> Self {
        Self {
            symbols: RwLock::new(snapshot.symbols),
            generator: Box::new(StandardGenerator::new()),
            metadata: RwLock::new(snapshot.metadata),
        }
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for Codebook<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> Clone for Codebook<T, DIM>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            symbols: RwLock::new(self.symbols.read().clone()),
            generator: Box::new(StandardGenerator::new()),
            metadata: RwLock::new(self.metadata.read().clone()),
        }
    }
}

/// Serializable codebook snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodebookSnapshot<T, const DIM: usize> {
    symbols: HashMap<String, TropicalDualClifford<T, DIM>>,
    metadata: CodebookMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_creation() {
        let codebook: Codebook<f64, 64> = Codebook::new();

        let paris = codebook.symbol("paris");
        let france = codebook.symbol("france");

        // Different symbols should be different
        assert!(paris.similarity(&france) < 0.5);

        // Same symbol should return same representation
        let paris2 = codebook.symbol("paris");
        assert!((paris.similarity(&paris2) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn explicit_registration() {
        let codebook: Codebook<f64, 64> = Codebook::new();

        let custom = TropicalDualClifford::random();
        codebook.register("custom", custom.clone());

        let retrieved = codebook.get("custom").unwrap();
        assert!((retrieved.similarity(&custom) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn nearest_symbol() {
        let codebook: Codebook<f64, 64> = Codebook::new();

        let paris = codebook.symbol("paris");
        let _france = codebook.symbol("france");
        let _berlin = codebook.symbol("berlin");

        // Query should find itself
        let (name, sim) = codebook.nearest(&paris).unwrap();
        assert_eq!(name, "paris");
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn symbol_with_properties() {
        let codebook: Codebook<f64, 8> = Codebook::new();

        // Create base symbol
        let _base = codebook.symbol("base");

        // Create orthogonal symbol
        let props = SymbolProperties {
            orthogonal_to: vec!["base".to_string()],
            ..Default::default()
        };

        let ortho = codebook.symbol_with_properties("ortho", &props);
        let base = codebook.get("base").unwrap();

        // Should be approximately orthogonal
        let sim = ortho.similarity(&base).abs();
        assert!(sim < 0.3);
    }
}
