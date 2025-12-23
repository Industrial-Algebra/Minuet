//! # Minuet: A Toolkit for Holographic Memory Systems
//!
//! > "The optical table for holographic computing."
//!
//! Minuet is an open-source Rust toolkit that extends `amari-holographic` with
//! higher-level abstractions for building cognitive memory systems. While
//! `amari-holographic` provides the core `BindingAlgebra` trait and algebra
//! implementations, Minuet adds:
//!
//! - **Memory Stores**: Sharded, partitioned, and layered memory configurations
//! - **Retrieval Strategies**: Direct, resonator-based, hybrid pipelines
//! - **Encoding Infrastructure**: Symbol codebooks, composite encoders
//! - **Capacity Management**: Monitoring, eviction policies, consolidation
//! - **Persistence**: Snapshots, journaling, crash recovery
//! - **Pipeline Composition**: Fluent builders for assembling memory systems
//!
//! Named after Star Trek's first sentient hologram, Minuet builds on the
//! foundations of `amari-holographic` to enable application-specific memory
//! architectures.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use minuet::prelude::*;
//! use amari_holographic::ProductCliffordAlgebra;
//!
//! type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions
//!
//! // Create a simple memory
//! let memory = SimpleMemory::<Algebra>::new();
//!
//! // Create symbols from codebook
//! let key = memory.symbol("paris");
//! let value = memory.symbol("france");
//!
//! // Store and retrieve
//! memory.store(&key, &value)?;
//! let result = memory.retrieve(&key)?;
//! ```
//!
//! ## Capacity Model
//!
//! Holographic memory has capacity O(DIM / log DIM). For typical dimensions:
//!
//! | Algebra Type | Dimension | Approx. Capacity |
//! |--------------|-----------|------------------|
//! | `ProductCliffordAlgebra<32>` | 256 | ~46 items |
//! | `ProductCliffordAlgebra<64>` | 512 | ~85 items |
//! | `ProductCliffordAlgebra<128>` | 1024 | ~147 items |
//!
//! For larger capacities, use [`store::ShardedStore`].

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]
// Intentional casts in numeric code - dimensions won't exceed 52-bit precision
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
// These are valid but would require significant refactoring
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
// Documentation style - backticks in tables/code references
#![allow(clippy::doc_markdown)]

// ============================================================================
// Re-exports from amari-holographic
// ============================================================================

/// Core re-exports from amari-holographic.
///
/// These are the fundamental types for holographic memory operations.
pub use amari_holographic::{
    AlgebraConfig, BindingAlgebra, HolographicMemory, ProductCliffordAlgebra,
    Resonator as HoloResonator, ResonatorConfig, RetrievalResult as HoloRetrievalResult,
};

// ============================================================================
// Core Modules
// ============================================================================

/// Core trait definitions for Minuet abstractions.
pub mod traits;

/// Error types and result aliases.
pub mod error;

/// Memory store implementations.
pub mod store;

/// Encoding infrastructure (codebooks, symbol encoders).
pub mod encoding;

/// Retrieval strategies (resonators, cleanup).
pub mod retrieval;

/// Capacity management (monitoring, eviction, consolidation).
pub mod capacity;

/// Pipeline composition (builders, executors).
pub mod pipeline;

/// Reference implementations for learning and simple use cases.
pub mod reference;

// ============================================================================
// Feature-gated Modules
// ============================================================================

// Persistence layer - to be implemented in a future version
// #[cfg(feature = "persistence")]
// pub mod persistence;

// ============================================================================
// Prelude
// ============================================================================

/// Prelude module for convenient imports.
///
/// Import everything commonly needed with:
/// ```rust,ignore
/// use minuet::prelude::*;
/// ```
pub mod prelude {
    // Core traits
    pub use crate::traits::{
        CapacityInfo, CapacityPolicy, CapacityWarning, CleanupResult, Codebook, Encoder,
        MemoryStore, MemoryTrace, PressureResponse, RetrievalContext, RetrievalResult, Retriever,
        StoreOptions, StoreReceipt, TraceCapacityInfo,
    };

    // Error types
    pub use crate::error::{MinuetError, MinuetResult, Result};

    // Store implementations
    pub use crate::store::{DenseTrace, ShardedStore, SimpleStore};

    // Encoding
    pub use crate::encoding::HashMapCodebook;

    // Retrieval
    pub use crate::retrieval::{DirectRetriever, ResonatorRetriever};

    // Pipeline
    pub use crate::pipeline::PipelineBuilder;

    // Reference implementations
    pub use crate::reference::SimpleMemory;

    // Re-export key algebra types from amari-holographic
    pub use amari_holographic::{BindingAlgebra, ProductCliffordAlgebra};
}

// ============================================================================
// Dimension Utilities
// ============================================================================

/// Compile-time dimension utilities.
///
/// Provides type-level computation for dimension-dependent constants.
pub mod dimensions {
    /// Compute theoretical capacity for a given dimension.
    ///
    /// Capacity scales as O(dim / log(dim)).
    ///
    /// # Examples
    /// - For dim=256: capacity ≈ 256/log(256) ≈ 46
    /// - For dim=1024: capacity ≈ 1024/log(1024) ≈ 147
    #[must_use]
    pub fn theoretical_capacity(dim: usize) -> usize {
        if dim <= 1 {
            return dim;
        }
        let log_dim = (dim as f64).ln();
        (dim as f64 / log_dim) as usize
    }

    /// Estimate signal-to-noise ratio for given dimension and item count.
    #[must_use]
    pub fn estimate_snr(dim: usize, item_count: usize) -> f64 {
        if item_count == 0 {
            return f64::INFINITY;
        }
        (dim as f64 / item_count as f64).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_utilities() {
        // ProductCliffordAlgebra<32> has 256 dimensions
        let cap = dimensions::theoretical_capacity(256);
        assert!(cap > 40 && cap < 50, "expected ~46, got {}", cap);

        // ProductCliffordAlgebra<128> has 1024 dimensions
        let cap = dimensions::theoretical_capacity(1024);
        assert!(cap > 140 && cap < 160, "expected ~147, got {}", cap);
    }

    #[test]
    fn snr_estimation() {
        let snr = dimensions::estimate_snr(256, 16);
        assert!((snr - 4.0).abs() < 0.01); // sqrt(256/16) = 4
    }
}
