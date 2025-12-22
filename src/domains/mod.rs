//! Domain-specific utilities for holographic memory.
//!
//! This module provides helpers for encoding domain-specific data
//! into holographic representations.
//!
//! # Available Domains
//!
//! - **Molecular**: SMILES strings, molecular fingerprints
//! - **Geometric**: SE(3) motor primitives, spatial relationships
//! - **Symbolic**: Code ASTs, symbolic expressions

pub mod geometric;
pub mod molecular;
pub mod symbolic;

use amari_fusion::TropicalDualClifford;

use crate::precision::MinuetFloat;

/// Trait for domain-specific encoders.
pub trait DomainEncoder<T: MinuetFloat, const DIM: usize> {
    /// The input type to encode.
    type Input;

    /// Encode an input into a holographic representation.
    fn encode(&self, input: &Self::Input) -> TropicalDualClifford<T, DIM>;

    /// Decode a holographic representation back to the domain (if possible).
    fn decode(&self, repr: &TropicalDualClifford<T, DIM>) -> Option<Self::Input>;
}

/// Trait for domain-specific similarity measures.
pub trait DomainSimilarity<T: MinuetFloat, const DIM: usize> {
    /// The input type.
    type Input;

    /// Compute domain-appropriate similarity between two items.
    fn similarity(&self, a: &Self::Input, b: &Self::Input) -> f64;

    /// Convert to holographic similarity.
    fn holographic_similarity(
        &self,
        a: &TropicalDualClifford<T, DIM>,
        b: &TropicalDualClifford<T, DIM>,
    ) -> f64;
}
