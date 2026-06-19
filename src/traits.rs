// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Core traits for Minuet holographic memory systems.
//!
//! These traits provide the abstraction layer between domain applications
//! and the underlying holographic algebra. All traits are generic over
//! `A: BindingAlgebra` to support any amari-holographic algebra backend.

use amari_holographic::BindingAlgebra;

use crate::error::MinuetResult;

// ============================================================================
// MemoryTrace - Fundamental storage unit
// ============================================================================

/// A holographic memory trace storing items in superposition.
///
/// The trace is the fundamental unit of holographic storage. Items are
/// added via bundling; retrieval unbinds a query against the trace.
///
/// # Implementors
///
/// - [`DenseTrace`](crate::store::DenseTrace): Standard dense representation
/// - Future: `SparseTrace`, `TieredTrace`
///
/// # Example
///
/// ```rust
/// # use minuet::prelude::*;
/// # type Algebra = ProductCliffordAlgebra<64>;
/// # fn main() {
/// let mut trace = DenseTrace::<Algebra>::new();
///
/// let item = Algebra::random_versor(2);
/// trace.add(&item, 1.0);
///
/// assert!(trace.similarity(&item) > 0.5);
/// # }
/// ```
pub trait MemoryTrace: Clone + Send + Sync {
    /// The binding algebra this trace uses.
    type Algebra: BindingAlgebra;

    // ========== Core Properties ==========

    /// The dimension of the underlying algebra.
    fn dimension(&self) -> usize;

    /// Number of items added to this trace.
    fn item_count(&self) -> usize;

    /// Whether the trace is empty.
    fn is_empty(&self) -> bool {
        self.item_count() == 0
    }

    /// Theoretical capacity before retrieval degrades.
    fn theoretical_capacity(&self) -> usize {
        let d = self.dimension() as f64;
        (d / d.ln().max(1.0)) as usize
    }

    /// Current capacity utilization [0, 1].
    fn utilization(&self) -> f64 {
        self.item_count() as f64 / self.theoretical_capacity().max(1) as f64
    }

    // ========== Modification ==========

    /// Add an item to the trace with given weight.
    fn add(&mut self, item: &Self::Algebra, weight: f64);

    /// Add an item with unit weight.
    fn add_unit(&mut self, item: &Self::Algebra) {
        self.add(item, 1.0);
    }

    /// Add multiple items with equal weights.
    fn add_all<'a, I>(&mut self, items: I)
    where
        I: IntoIterator<Item = &'a Self::Algebra>,
        Self::Algebra: 'a,
    {
        for item in items {
            self.add_unit(item);
        }
    }

    /// Bundle another trace into this one.
    fn merge(&mut self, other: &Self, weight: f64);

    /// Clear the trace.
    fn clear(&mut self);

    // ========== Query ==========

    /// Compute similarity between query and trace.
    fn similarity(&self, query: &Self::Algebra) -> f64;

    /// Unbind query from trace (raw retrieval).
    fn unbind(&self, query: &Self::Algebra) -> Self::Algebra;

    /// Get the raw trace representation.
    fn as_algebra(&self) -> Self::Algebra;

    // ========== Diagnostics ==========

    /// Estimate current signal-to-noise ratio.
    fn estimated_snr(&self) -> f64 {
        if self.item_count() == 0 {
            return f64::INFINITY;
        }
        (self.dimension() as f64 / self.item_count() as f64).sqrt()
    }

    /// Whether the trace is near capacity.
    fn near_capacity(&self, threshold: f64) -> bool {
        self.utilization() > threshold
    }
}

// ============================================================================
// MemoryStore - Higher-level storage
// ============================================================================

/// A store managing holographic memory traces.
///
/// Stores provide higher-level operations on top of traces:
/// - Key-value storage (bind key with value, store in trace)
/// - Structured queries
/// - Capacity management
/// - Optional persistence
///
/// # Implementors
///
/// - [`SimpleStore`](crate::store::SimpleStore): Single trace, minimal overhead
/// - [`ShardedStore`](crate::store::ShardedStore): Multiple traces, hash-sharded
/// - Future: `PartitionedStore`, `LayeredStore`
pub trait MemoryStore: Send + Sync {
    /// The trace type this store manages.
    type Trace: MemoryTrace;

    /// Algebra type (convenience alias).
    type Algebra: BindingAlgebra;

    // ========== Storage ==========

    /// Store a key-value association.
    fn store(&self, key: &Self::Algebra, value: &Self::Algebra) -> MinuetResult<StoreReceipt>;

    /// Store with explicit options.
    fn store_with_options(
        &self,
        key: &Self::Algebra,
        value: &Self::Algebra,
        options: StoreOptions,
    ) -> MinuetResult<StoreReceipt>;

    /// Store multiple associations.
    fn store_batch(
        &self,
        pairs: &[(Self::Algebra, Self::Algebra)],
    ) -> MinuetResult<Vec<StoreReceipt>>;

    // ========== Retrieval ==========

    /// Retrieve value associated with key.
    fn retrieve(&self, key: &Self::Algebra) -> MinuetResult<RetrievalResult<Self::Algebra>>;

    // ========== Management ==========

    /// Get capacity information.
    fn capacity_info(&self) -> CapacityInfo;

    /// Clear all stored associations.
    fn clear(&self) -> MinuetResult<()>;

    // ========== Trace Access ==========

    /// Number of traces in this store.
    fn trace_count(&self) -> usize;

    /// Total item count across all traces.
    fn total_items(&self) -> usize;
}

/// Options for store operations.
#[derive(Clone, Debug, Default)]
pub struct StoreOptions {
    /// Target partition (for partitioned stores).
    pub partition: Option<String>,
    /// Weight for this item.
    pub weight: f64,
    /// Source ID for attribution tracking.
    pub source_id: Option<u64>,
    /// Skip capacity checks (dangerous).
    pub force: bool,
}

impl StoreOptions {
    /// Create default options with unit weight.
    pub fn new() -> Self {
        Self {
            weight: 1.0,
            ..Default::default()
        }
    }

    /// Set target partition.
    pub fn partition(mut self, name: impl Into<String>) -> Self {
        self.partition = Some(name.into());
        self
    }

    /// Set weight.
    pub fn weight(mut self, w: f64) -> Self {
        self.weight = w;
        self
    }

    /// Set source ID for attribution.
    pub fn source_id(mut self, id: u64) -> Self {
        self.source_id = Some(id);
        self
    }

    /// Force storage even at capacity.
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Receipt from a store operation.
#[derive(Clone, Debug)]
pub struct StoreReceipt {
    /// Unique ID for this storage operation.
    pub id: u64,
    /// Estimated SNR after storage.
    pub post_snr: f64,
    /// Capacity warning if approaching limits.
    pub warning: Option<CapacityWarning>,
    /// Which trace/partition received the item.
    pub location: String,
}

/// Result of a retrieval operation.
#[derive(Clone, Debug)]
pub struct RetrievalResult<A> {
    /// Retrieved value.
    pub value: A,
    /// Confidence score [0, 1].
    pub confidence: f64,
    /// Attribution (source_id, contribution).
    pub attribution: Vec<(u64, f64)>,
}

/// Capacity information.
#[derive(Clone, Debug)]
pub struct CapacityInfo {
    /// Total items across all traces.
    pub total_items: usize,
    /// Theoretical capacity.
    pub theoretical_capacity: usize,
    /// Overall utilization [0, 1].
    pub utilization: f64,
    /// Estimated SNR.
    pub estimated_snr: f64,
    /// Per-trace information.
    pub per_trace: Vec<TraceCapacityInfo>,
}

/// Capacity info for a single trace.
#[derive(Clone, Debug)]
pub struct TraceCapacityInfo {
    /// Trace name/identifier.
    pub name: String,
    /// Items in this trace.
    pub items: usize,
    /// Trace capacity.
    pub capacity: usize,
    /// Trace utilization.
    pub utilization: f64,
}

/// Capacity warning types.
#[derive(Clone, Debug)]
pub enum CapacityWarning {
    /// Approaching capacity threshold.
    ApproachingCapacity {
        /// Current utilization (0.0 - 1.0).
        utilization: f64,
    },
    /// At capacity.
    AtCapacity,
    /// SNR has degraded below threshold.
    SNRDegraded {
        /// Current SNR value.
        snr: f64,
        /// Threshold that triggered the warning.
        threshold: f64,
    },
}

// ============================================================================
// Retriever - Cleanup strategies
// ============================================================================

/// A retrieval strategy for holographic memory.
///
/// Retrievers take raw unbind results and clean them up, potentially
/// using a codebook, resonator network, or other mechanism.
///
/// # Implementors
///
/// - [`DirectRetriever`](crate::retrieval::DirectRetriever): Return raw result
/// - [`ResonatorRetriever`](crate::retrieval::ResonatorRetriever): Cleanup via resonator
/// - Future: `AnnealedRetriever`, `HybridRetriever`
pub trait Retriever: Send + Sync {
    /// The algebra type.
    type Algebra: BindingAlgebra;

    /// Clean up a raw retrieval result.
    fn cleanup(
        &self,
        raw: &Self::Algebra,
        context: &RetrievalContext<Self::Algebra>,
    ) -> MinuetResult<CleanupResult<Self::Algebra>>;

    /// Batch cleanup for multiple results.
    fn cleanup_batch(
        &self,
        raws: &[Self::Algebra],
        context: &RetrievalContext<Self::Algebra>,
    ) -> MinuetResult<Vec<CleanupResult<Self::Algebra>>> {
        raws.iter().map(|r| self.cleanup(r, context)).collect()
    }
}

/// Context provided to retrievers.
#[derive(Clone)]
pub struct RetrievalContext<A> {
    /// Optional codebook for cleanup.
    pub codebook: Option<Vec<A>>,
    /// Temperature parameter (beta). Higher = harder selection.
    pub temperature: f64,
    /// Maximum cleanup iterations.
    pub max_iterations: usize,
    /// Convergence threshold.
    pub convergence_threshold: f64,
}

impl<A> Default for RetrievalContext<A> {
    fn default() -> Self {
        Self {
            codebook: None,
            temperature: f64::INFINITY, // Hard by default
            max_iterations: 50,
            convergence_threshold: 0.999,
        }
    }
}

impl<A> RetrievalContext<A> {
    /// Add a codebook for cleanup.
    pub fn with_codebook(mut self, codebook: Vec<A>) -> Self {
        self.codebook = Some(codebook);
        self
    }

    /// Set temperature parameter.
    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = t;
        self
    }

    /// Soft retrieval context (low temperature).
    pub fn soft() -> Self {
        Self {
            temperature: 1.0,
            ..Default::default()
        }
    }

    /// Hard retrieval context (high temperature).
    pub fn hard() -> Self {
        Self {
            temperature: f64::INFINITY,
            ..Default::default()
        }
    }
}

/// Result of cleanup operation.
#[derive(Clone, Debug)]
pub struct CleanupResult<A> {
    /// Cleaned value.
    pub value: A,
    /// Confidence in cleanup [0, 1].
    pub confidence: f64,
    /// Number of iterations (if iterative).
    pub iterations: usize,
    /// Whether cleanup converged.
    pub converged: bool,
    /// Index in codebook if matched.
    pub codebook_match: Option<usize>,
}

// ============================================================================
// Encoder - Domain encoding
// ============================================================================

/// Encodes domain objects into algebraic representations.
///
/// Encoders are the bridge between your domain (molecules, code, text)
/// and the holographic representation space.
///
/// # Design Guidelines
///
/// - Encoders should be deterministic (same input -> same output)
/// - Similar inputs should produce similar outputs
/// - Compositional structure in input should reflect in output
///
/// # Implementors
///
/// - Future: `SymbolEncoder` - Map symbols to random vectors
/// - Future: `CompositeEncoder` - Build from parts
/// - Custom implementations for your domain
pub trait Encoder: Send + Sync {
    /// Input type this encoder accepts.
    type Input: ?Sized;

    /// The algebra type.
    type Algebra: BindingAlgebra;

    /// Encode an input into algebraic form.
    fn encode(&self, input: &Self::Input) -> MinuetResult<Self::Algebra>;

    /// Batch encoding.
    fn encode_batch<'a, I>(&self, inputs: I) -> MinuetResult<Vec<Self::Algebra>>
    where
        I: IntoIterator<Item = &'a Self::Input>,
        Self::Input: 'a,
    {
        inputs.into_iter().map(|i| self.encode(i)).collect()
    }

    /// Attempt to decode back to input (if supported).
    fn decode(&self, _repr: &Self::Algebra) -> MinuetResult<Option<Self::Input>>
    where
        Self::Input: Sized,
    {
        Ok(None) // Default: decoding not supported
    }
}

// ============================================================================
// Codebook - Symbol vocabularies
// ============================================================================

/// A vocabulary of symbols with stable representations.
///
/// Codebooks ensure consistent symbol-to-vector mapping across
/// encoding operations. They also provide cleanup targets for
/// resonator networks.
///
/// # Implementors
///
/// - [`HashMapCodebook`](crate::encoding::HashMapCodebook): In-memory codebook
/// - Future: `PersistentCodebook`, `HierarchicalCodebook`
pub trait Codebook: Send + Sync {
    /// The algebra type.
    type Algebra: BindingAlgebra;

    /// Get or create a symbol representation.
    fn symbol(&self, name: &str) -> Self::Algebra;

    /// Get symbol if it exists.
    fn get(&self, name: &str) -> Option<Self::Algebra>;

    /// Check if symbol exists.
    fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Register a specific representation for a symbol.
    fn register(&self, name: &str, repr: Self::Algebra) -> MinuetResult<()>;

    /// Number of symbols in codebook.
    fn len(&self) -> usize;

    /// Whether codebook is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all symbol representations (for resonator cleanup).
    fn all_symbols(&self) -> Vec<Self::Algebra>;

    /// Get all symbol names.
    fn all_names(&self) -> Vec<String>;

    /// Find closest symbol to a representation.
    fn closest(&self, repr: &Self::Algebra) -> Option<(String, f64)>;
}

// ============================================================================
// CapacityPolicy - Capacity management
// ============================================================================

/// Policy for managing memory capacity.
///
/// Capacity policies decide what to do when memory is full
/// or approaching capacity limits.
///
/// # Implementors
///
/// - [`RejectPolicy`](crate::capacity::RejectPolicy): Refuse new items
/// - Future: `EvictOldestPolicy` - Remove oldest
/// - Future: `EvictLRUPolicy`, `ConsolidatePolicy`
pub trait CapacityPolicy: Send + Sync {
    /// Check if store can accept more items.
    fn can_accept(&self, info: &CapacityInfo) -> bool;

    /// Get warning threshold [0, 1].
    fn warning_threshold(&self) -> f64 {
        0.8
    }

    /// Get critical threshold [0, 1].
    fn critical_threshold(&self) -> f64 {
        0.95
    }
}

/// Response to capacity pressure.
#[derive(Clone, Debug)]
pub enum PressureResponse {
    /// Accepted, no action needed.
    Accepted,
    /// Items were evicted.
    Evicted {
        /// Number of items evicted.
        count: usize,
    },
    /// Store was consolidated.
    Consolidated {
        /// Item count before consolidation.
        items_before: usize,
        /// Item count after consolidation.
        items_after: usize,
    },
    /// Item rejected.
    Rejected {
        /// Reason for rejection.
        reason: String,
    },
}
