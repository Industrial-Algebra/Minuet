//! # Minuet: A Holographic Database
//!
//! Minuet is a holographic database built on `amari-fusion`'s tropical-dual-Clifford
//! algebra. Named after Star Trek's first sentient hologram, Minuet provides memory
//! that participates in cognition rather than merely serving it.
//!
//! ## Core Proposition
//!
//! Retrieval is a native algebraic operation, not index lookup with a translation layer.
//! Queries are pattern completions in the same representational space as stored knowledge.
//!
//! ## Key Features
//!
//! - **Compositional associative memory** where relationships are first-class
//! - **Analogical queries** like "find X related to Y as A is related to B" as single operations
//! - **Graceful degradation** under noise, partial queries, and capacity pressure
//! - **Type-safe algebra** with phantom types tracking invertibility, normalization, and grade
//!
//! ## Example
//!
//! ```rust,ignore
//! use minuet::{MemoryStore, Query, Codebook};
//!
//! // Create a memory store with 256-dimensional representations
//! let memory = MemoryStore::<f64, 256>::new();
//! let codebook = Codebook::new();
//!
//! // Create symbols
//! let paris = codebook.symbol("paris");
//! let france = codebook.symbol("france");
//! let berlin = codebook.symbol("berlin");
//!
//! // Store: paris is-capital-of france
//! memory.store(&paris, &france)?;
//!
//! // Query: what is berlin the capital of?
//! // (analogy: berlin:X :: paris:france)
//! let query = Query::analogy(paris, france, berlin);
//! let result = memory.query(query)?;
//! ```
//!
//! ## Capacity Model
//!
//! Holographic memory has capacity O(DIM / log DIM). For typical dimensions:
//!
//! | Dimension | Approx. Capacity |
//! |-----------|------------------|
//! | 256       | ~45 items        |
//! | 1024      | ~150 items       |
//! | 4096      | ~500 items       |
//!
//! For larger capacities, use [`parallel::ShardedMemory`].

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]

#[cfg(feature = "contracts")]
use creusot_contracts::*;

// Re-export core types from amari-fusion
pub use amari_fusion::holographic::{Bindable, RetrievalResult, TropicalDualClifford};

// Core modules
pub mod binding;
pub mod error;
pub mod memory;
pub mod parallel;
pub mod retrieval;

// Feature-gated modules
#[cfg(feature = "persistence")]
pub mod persistence;

#[cfg(feature = "gpu")]
pub mod gpu;

pub mod domains;

// Public re-exports for convenience
pub use binding::{Codebook, SymbolGenerator, Transform};
pub use error::{CapacityWarning, MinuetError, Result};
pub use memory::{CapacityInfo, MemoryStore, MemoryTrace, Query, QueryResult, StoreReceipt};
pub use parallel::{batch, ShardedMemory};
pub use retrieval::{Attribution, Resonator, Temperature};

/// Precision traits for numeric operations.
///
/// This module provides abstractions over floating-point types to allow
/// seamless switching between f32, f64, and high-precision types.
pub mod precision {
    use num_traits::{Float, FromPrimitive, NumCast, ToPrimitive};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::iter::Sum;
    use std::ops::{Add, Div, Mul, Neg, Sub};

    /// Trait bounds for numeric types usable in Minuet.
    ///
    /// This trait encapsulates all the numeric operations needed for
    /// holographic memory operations.
    pub trait MinuetFloat:
        Float
        + FromPrimitive
        + ToPrimitive
        + NumCast
        + Sum
        + Debug
        + Clone
        + Copy
        + Send
        + Sync
        + Default
        + Serialize
        + DeserializeOwned
        + Add<Output = Self>
        + Sub<Output = Self>
        + Mul<Output = Self>
        + Div<Output = Self>
        + Neg<Output = Self>
        + 'static
    {
        /// Machine epsilon for this type.
        const EPSILON: Self;

        /// Minimum positive normal value.
        const MIN_POSITIVE: Self;

        /// Maximum finite value.
        const MAX: Self;

        /// The constant pi.
        const PI: Self;

        /// The constant e.
        const E: Self;

        /// Default tolerance for approximate equality.
        fn default_tolerance() -> Self;

        /// Check if two values are approximately equal.
        fn approx_eq(self, other: Self, tolerance: Self) -> bool {
            (self - other).abs() <= tolerance
        }
    }

    impl MinuetFloat for f32 {
        const EPSILON: Self = f32::EPSILON;
        const MIN_POSITIVE: Self = f32::MIN_POSITIVE;
        const MAX: Self = f32::MAX;
        const PI: Self = std::f32::consts::PI;
        const E: Self = std::f32::consts::E;

        fn default_tolerance() -> Self {
            1e-5
        }
    }

    impl MinuetFloat for f64 {
        const EPSILON: Self = f64::EPSILON;
        const MIN_POSITIVE: Self = f64::MIN_POSITIVE;
        const MAX: Self = f64::MAX;
        const PI: Self = std::f64::consts::PI;
        const E: Self = std::f64::consts::E;

        fn default_tolerance() -> Self {
            1e-10
        }
    }

    #[cfg(feature = "high-precision")]
    pub use num_bigfloat::BigFloat;
}

/// Compile-time dimension utilities.
///
/// Provides type-level computation for dimension-dependent constants.
pub mod dimensions {
    /// Compute theoretical capacity for a given dimension.
    ///
    /// Capacity scales as O(DIM / log DIM).
    #[must_use]
    pub const fn theoretical_capacity(dim: usize) -> usize {
        // log2(dim) approximation for const context
        let log_dim = (usize::BITS - dim.leading_zeros()) as usize;
        if log_dim == 0 {
            0
        } else {
            dim / log_dim
        }
    }

    /// Check if dimension is a power of two (preferred for efficiency).
    #[must_use]
    pub const fn is_power_of_two(dim: usize) -> bool {
        dim > 0 && (dim & (dim - 1)) == 0
    }

    /// Get the grade count for a given dimension (number of basis blades).
    #[must_use]
    pub const fn grade_count(dim: usize) -> usize {
        dim + 1
    }

    /// Get the total number of basis elements (2^dim for Clifford algebra).
    #[must_use]
    pub const fn basis_count(dim: usize) -> usize {
        1 << dim
    }
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::binding::{Codebook, Transform};
    pub use crate::error::{MinuetError, Result};
    pub use crate::memory::{MemoryStore, MemoryTrace, Query};
    pub use crate::precision::MinuetFloat;
    pub use crate::retrieval::{Resonator, Temperature};

    // Re-export key amari-fusion types
    pub use amari_fusion::holographic::{Bindable, TropicalDualClifford};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_utilities() {
        assert_eq!(dimensions::theoretical_capacity(256), 32); // 256/8
        assert_eq!(dimensions::theoretical_capacity(1024), 102); // 1024/10
        assert!(dimensions::is_power_of_two(256));
        assert!(!dimensions::is_power_of_two(257));
        assert_eq!(dimensions::grade_count(8), 9);
        assert_eq!(dimensions::basis_count(8), 256);
    }

    #[test]
    fn float_tolerance() {
        use precision::MinuetFloat;

        let a: f64 = 1.0;
        let b: f64 = 1.0 + 1e-11;
        assert!(a.approx_eq(b, f64::default_tolerance()));

        let c: f64 = 1.0 + 1e-9;
        assert!(!a.approx_eq(c, f64::default_tolerance()));
    }
}
