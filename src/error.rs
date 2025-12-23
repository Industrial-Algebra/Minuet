//! Error types for Minuet holographic memory operations.
//!
//! This module defines a comprehensive error hierarchy for all operations
//! in the Minuet system, from low-level algebraic failures to high-level
//! query errors.

use std::fmt;
use thiserror::Error;

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

    /// Capacity exceeded (store cannot accept more items).
    #[error("Capacity exceeded: store cannot accept more items")]
    CapacityExceeded,

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

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// I/O error during persistence operations.
    #[error("Persistence error: {0}")]
    Persistence(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[cfg(feature = "serde")]
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

    /// Invalid grade for the algebra dimension.
    #[error("Invalid grade {grade} for dimension {dim}")]
    InvalidGrade {
        /// The requested grade
        grade: usize,
        /// The dimension of the algebra
        dim: usize,
    },

    /// Merge operation failed.
    #[error("Merge failed: {0}")]
    MergeFailed(String),

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

    /// Algebra operation error from amari-holographic.
    #[error("Algebra error: {0}")]
    Algebra(String),
}

/// Result type alias for Minuet operations.
pub type MinuetResult<T> = std::result::Result<T, MinuetError>;

/// Convenience alias
pub type Result<T> = MinuetResult<T>;

impl MinuetError {
    /// Create an algebra error from any displayable source.
    pub fn algebra(msg: impl fmt::Display) -> Self {
        Self::Algebra(msg.to_string())
    }

    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a not-implemented error.
    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self::NotImplemented {
            feature: feature.into(),
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
    fn algebra_error_from_string() {
        let err = MinuetError::algebra("test error");
        assert!(err.to_string().contains("test error"));
    }
}
