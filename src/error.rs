//! Error types for Minuet holographic database operations.
//!
//! This module defines a comprehensive error hierarchy for all operations
//! in the Minuet system, from low-level algebraic failures to high-level
//! query errors.

use std::fmt;
use thiserror::Error;

#[cfg(feature = "contracts")]
use creusot_contracts::*;

/// Primary error type for Minuet operations.
///
/// All errors carry sufficient context for debugging and recovery,
/// including algebraic invariants that were violated.
#[derive(Debug, Error)]
pub enum MinuetError {
    /// Memory has reached capacity; signal-to-noise ratio is too low.
    #[error("Memory at capacity: SNR {snr:.3} below threshold {threshold:.3}")]
    AtCapacity {
        /// Current signal-to-noise ratio
        snr: f64,
        /// Minimum acceptable SNR threshold
        threshold: f64,
    },

    /// Cannot compute inverse due to near-singular element.
    #[error("Cannot compute inverse: {message}")]
    SingularInverse {
        /// Description of why the inverse failed
        message: String,
    },

    /// Normalization failed due to zero or near-zero norm.
    #[error("Normalization failed: {message}")]
    NormalizationFailed {
        /// Description of the normalization failure
        message: String,
    },

    /// Dimension mismatch in algebraic operation.
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimensionality
        expected: usize,
        /// Actual dimensionality found
        actual: usize,
    },

    /// Resonator cleanup network failed to converge.
    #[error("Resonator failed to converge after {iterations} iterations (similarity: {final_similarity:.3})")]
    ResonatorDidNotConverge {
        /// Number of iterations attempted
        iterations: usize,
        /// Best similarity achieved
        final_similarity: f64,
    },

    /// Symbol not found in codebook.
    #[error("Symbol not found in codebook: {0}")]
    SymbolNotFound(String),

    /// Invalid codebook state.
    #[error("Codebook invariant violated: {0}")]
    CodebookInvariant(String),

    /// I/O error during persistence operations.
    #[error("Persistence error: {0}")]
    Persistence(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// Invalid query specification.
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Temperature parameter out of valid range.
    #[error("Invalid temperature beta={beta}: must be in range ({min}, {max})")]
    InvalidTemperature {
        /// The provided temperature value
        beta: f64,
        /// Minimum valid temperature (exclusive)
        min: f64,
        /// Maximum valid temperature (exclusive)
        max: f64,
    },

    /// Algebraic constraint violation.
    #[error("Algebraic constraint violated: {constraint} (value: {value:.6})")]
    AlgebraicConstraint {
        /// Description of the violated constraint
        constraint: String,
        /// The value that violated the constraint
        value: f64,
    },

    /// Grade projection error.
    #[error("Invalid grade {grade} for dimension {dim}")]
    InvalidGrade {
        /// The requested grade
        grade: usize,
        /// The dimension of the algebra
        dim: usize,
    },

    /// Transform extraction failed.
    #[error("Transform extraction failed: {0}")]
    TransformExtraction(String),

    /// Merge operation failed.
    #[error("Merge failed: {0}")]
    MergeFailed(String),

    /// GPU backend error (only with gpu feature).
    #[cfg(feature = "gpu")]
    #[error("GPU error: {0}")]
    Gpu(String),

    /// Distributed operation error (only with distributed feature).
    #[cfg(feature = "distributed")]
    #[error("Distributed error: {0}")]
    Distributed(String),

    /// Recovery from crash failed.
    #[cfg(feature = "persistence")]
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    /// Feature not yet implemented.
    #[error("Not implemented: {feature}")]
    NotImplemented {
        /// Description of the unimplemented feature
        feature: String,
    },
}

/// Result type alias for Minuet operations.
pub type Result<T> = std::result::Result<T, MinuetError>;

/// Capacity warning levels for proactive management.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CapacityWarning {
    /// Approaching capacity limit (SNR degrading).
    Approaching {
        /// Current utilization as fraction of theoretical capacity
        utilization: f64,
        /// Estimated remaining stores before threshold
        remaining_stores: usize,
    },
    /// At capacity, further stores will fail.
    Critical {
        /// Current SNR
        snr: f64,
    },
}

impl fmt::Display for CapacityWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapacityWarning::Approaching {
                utilization,
                remaining_stores,
            } => {
                write!(
                    f,
                    "Approaching capacity: {:.1}% utilized, ~{} stores remaining",
                    utilization * 100.0,
                    remaining_stores
                )
            }
            CapacityWarning::Critical { snr } => {
                write!(f, "Critical: SNR at {:.3}, capacity exhausted", snr)
            }
        }
    }
}

/// Marker types for tracking algebraic state at the type level.
pub mod markers {
    use std::marker::PhantomData;

    /// Marker indicating an element has been verified as invertible.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Invertible;

    /// Marker indicating an element may not be invertible.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MaybeInvertible;

    /// Marker indicating an element is normalized (unit magnitude).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Normalized;

    /// Marker indicating an element has arbitrary magnitude.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Unnormalized;

    /// Marker indicating an element is a pure grade (k-blade).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PureGrade<const K: usize>;

    /// Marker indicating an element is a mixed-grade multivector.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MixedGrade;

    /// Marker indicating an element is a versor (product of vectors).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Versor;

    /// Marker indicating an element may not be a versor.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct GeneralMultivector;

    /// Phantom wrapper to carry type-level markers without runtime cost.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Marked<T, Invertibility, Normalization, GradeStructure> {
        value: T,
        _invertibility: PhantomData<Invertibility>,
        _normalization: PhantomData<Normalization>,
        _grade: PhantomData<GradeStructure>,
    }

    impl<T, I, N, G> Marked<T, I, N, G> {
        /// Create a new marked value.
        pub fn new(value: T) -> Self {
            Self {
                value,
                _invertibility: PhantomData,
                _normalization: PhantomData,
                _grade: PhantomData,
            }
        }

        /// Extract the inner value, discarding markers.
        pub fn into_inner(self) -> T {
            self.value
        }

        /// Reference to the inner value.
        pub fn inner(&self) -> &T {
            &self.value
        }

        /// Mutable reference to the inner value (use with care).
        pub fn inner_mut(&mut self) -> &mut T {
            &mut self.value
        }

        /// Transmute markers (unsafe operation, use only when semantically valid).
        pub fn transmute_markers<I2, N2, G2>(self) -> Marked<T, I2, N2, G2> {
            Marked {
                value: self.value,
                _invertibility: PhantomData,
                _normalization: PhantomData,
                _grade: PhantomData,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_formatting() {
        let err = MinuetError::AtCapacity {
            snr: 0.123,
            threshold: 0.5,
        };
        assert!(err.to_string().contains("0.123"));
        assert!(err.to_string().contains("0.500"));
    }

    #[test]
    fn capacity_warning_display() {
        let warn = CapacityWarning::Approaching {
            utilization: 0.85,
            remaining_stores: 42,
        };
        let msg = warn.to_string();
        assert!(msg.contains("85.0%"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn phantom_markers_zero_cost() {
        use markers::*;
        use std::mem::size_of;

        // Markers should be zero-sized
        assert_eq!(size_of::<Invertible>(), 0);
        assert_eq!(size_of::<Normalized>(), 0);
        assert_eq!(size_of::<PureGrade<2>>(), 0);

        // Marked wrapper should not increase size
        assert_eq!(
            size_of::<Marked<f64, Invertible, Normalized, Versor>>(),
            size_of::<f64>()
        );
    }
}
