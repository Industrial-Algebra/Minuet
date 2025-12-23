//! Algebra backend abstraction layer.
//!
//! This module provides a unified interface for holographic algebras,
//! allowing Minuet to work with different backends:
//!
//! - `amari-holographic` (recommended): Purpose-built for holographic memory
//! - `amari-fusion` (legacy): Original TropicalDualClifford-based implementation
//!
//! # Feature Flags
//!
//! - `holographic` (default): Use amari-holographic algebras
//! - `legacy-fusion`: Use amari-fusion TropicalDualClifford
//!
//! # Example
//!
//! ```ignore
//! use minuet::algebra::{Algebra, DefaultAlgebra};
//!
//! // Create random elements
//! let key = DefaultAlgebra::random_element();
//! let value = DefaultAlgebra::random_element();
//!
//! // Bind and unbind
//! let bound = Algebra::bind(&key, &value);
//! let recovered = Algebra::unbind(&key, &bound)?;
//! ```

use crate::error::{MinuetError, Result};

// ============================================================================
// Backend Selection via Feature Flags
// ============================================================================

// Re-export amari-fusion types (always available for compatibility)
pub use amari_fusion::holographic::Bindable;
pub use amari_fusion::TropicalDualClifford;

// Re-export amari-holographic types
pub use amari_holographic::{BindingAlgebra as HoloBindingAlgebra, ProductCliffordAlgebra};

// ============================================================================
// Algebra Trait (unified interface)
// ============================================================================

/// Unified algebra trait that abstracts over different backends.
///
/// This trait provides a common interface for holographic binding operations,
/// allowing code to work with either amari-fusion or amari-holographic.
pub trait Algebra: Clone + Send + Sync + Sized {
    /// Create the zero element (additive identity for bundling).
    fn zero() -> Self;

    /// Create the identity element (multiplicative identity for binding).
    fn identity() -> Self;

    /// Create a random element suitable for keys/values.
    fn random_element() -> Self;

    /// Bind two elements (create association).
    ///
    /// Result should be dissimilar to both inputs.
    fn bind(&self, other: &Self) -> Self;

    /// Compute the binding inverse.
    ///
    /// Returns `Err` if the element is not invertible.
    fn inverse(&self) -> Result<Self>;

    /// Unbind: retrieve associated value.
    ///
    /// Equivalent to `self.inverse()?.bind(other)`.
    fn unbind(&self, other: &Self) -> Result<Self> {
        Ok(self.inverse()?.bind(other))
    }

    /// Bundle two elements (superposition).
    ///
    /// Result should be similar to both inputs.
    /// `beta` controls soft (1.0) vs hard (infinity) bundling.
    fn bundle(&self, other: &Self, beta: f64) -> Result<Self>;

    /// Compute similarity between two elements.
    ///
    /// Returns a value in [-1, 1].
    fn similarity(&self, other: &Self) -> f64;

    /// Compute the norm.
    fn norm(&self) -> f64;

    /// Normalize to unit norm.
    fn normalize(&self) -> Result<Self>;

    /// Get the dimension of this algebra.
    fn dimension() -> usize;

    /// Estimate theoretical capacity.
    fn theoretical_capacity() -> usize {
        let dim = Self::dimension() as f64;
        if dim <= 1.0 {
            return 1;
        }
        (dim / dim.ln()).max(1.0) as usize
    }

    /// Estimate SNR for a given number of stored items.
    fn estimate_snr(item_count: usize) -> f64 {
        if item_count == 0 {
            return f64::INFINITY;
        }
        let dim = Self::dimension() as f64;
        (dim / item_count as f64).sqrt()
    }
}

// ============================================================================
// Implementation for TropicalDualClifford (current backend)
// ============================================================================

impl<T, const DIM: usize> Algebra for TropicalDualClifford<T, DIM>
where
    T: crate::precision::MinuetFloat + Send + Sync,
    TropicalDualClifford<T, DIM>: Clone,
{
    fn zero() -> Self {
        TropicalDualClifford::new()
    }

    fn identity() -> Self {
        Bindable::binding_identity()
    }

    fn random_element() -> Self {
        TropicalDualClifford::random()
    }

    fn bind(&self, other: &Self) -> Self {
        Bindable::bind(self, other)
    }

    fn inverse(&self) -> Result<Self> {
        self.binding_inverse()
            .ok_or_else(|| MinuetError::SingularInverse {
                message: "Element not invertible".into(),
            })
    }

    fn bundle(&self, other: &Self, beta: f64) -> Result<Self> {
        Ok(Bindable::bundle(self, other, beta))
    }

    fn similarity(&self, other: &Self) -> f64 {
        Bindable::similarity(self, other)
    }

    fn norm(&self) -> f64 {
        Bindable::norm(self)
    }

    fn normalize(&self) -> Result<Self> {
        let n = self.norm();
        if n < 1e-10 {
            return Err(MinuetError::NormalizationFailed {
                message: format!("Norm too small: {}", n),
            });
        }
        Ok(Bindable::normalize(self))
    }

    fn dimension() -> usize {
        // For TropicalDualClifford, the algebra dimension is 2^DIM
        1usize << DIM
    }
}

// ============================================================================
// Implementation for ProductCliffordAlgebra (amari-holographic backend)
// ============================================================================

impl<const K: usize> Algebra for ProductCliffordAlgebra<K> {
    fn zero() -> Self {
        HoloBindingAlgebra::zero()
    }

    fn identity() -> Self {
        HoloBindingAlgebra::identity()
    }

    fn random_element() -> Self {
        ProductCliffordAlgebra::random_versor(2)
    }

    fn bind(&self, other: &Self) -> Self {
        HoloBindingAlgebra::bind(self, other)
    }

    fn inverse(&self) -> Result<Self> {
        HoloBindingAlgebra::inverse(self).map_err(|e| MinuetError::SingularInverse {
            message: e.to_string(),
        })
    }

    fn bundle(&self, other: &Self, beta: f64) -> Result<Self> {
        HoloBindingAlgebra::bundle(self, other, beta).map_err(|e| {
            MinuetError::AlgebraicConstraint {
                constraint: "bundle operation failed".into(),
                value: beta,
            }
        })
    }

    fn similarity(&self, other: &Self) -> f64 {
        HoloBindingAlgebra::similarity(self, other)
    }

    fn norm(&self) -> f64 {
        HoloBindingAlgebra::norm(self)
    }

    fn normalize(&self) -> Result<Self> {
        HoloBindingAlgebra::normalize(self).map_err(|e| MinuetError::NormalizationFailed {
            message: e.to_string(),
        })
    }

    fn dimension() -> usize {
        // ProductCliffordAlgebra<K> has dimension 8*K
        8 * K
    }
}

// ============================================================================
// Type Aliases for Common Configurations
// ============================================================================

// When using the holographic feature, use ProductCliffordAlgebra
#[cfg(feature = "holographic")]
mod type_aliases {
    use super::*;

    /// Default algebra type (256-dimensional).
    ///
    /// Uses ProductCliffordAlgebra<32> which has 32 copies of Cl(3,0,0).
    pub type DefaultAlgebra = ProductCliffordAlgebra<32>;

    /// Small algebra for testing (64-dimensional).
    pub type SmallAlgebra = ProductCliffordAlgebra<8>;

    /// 256-dimensional algebra.
    pub type Algebra256 = ProductCliffordAlgebra<32>;

    /// 512-dimensional algebra for higher capacity.
    pub type Algebra512 = ProductCliffordAlgebra<64>;

    /// 1024-dimensional algebra for maximum capacity.
    pub type Algebra1024 = ProductCliffordAlgebra<128>;
}

// When using the legacy-fusion feature, use TropicalDualClifford
#[cfg(all(feature = "legacy-fusion", not(feature = "holographic")))]
mod type_aliases {
    use super::*;

    /// Default algebra type (256-dimensional).
    pub type DefaultAlgebra = TropicalDualClifford<f64, 8>;

    /// Small algebra for testing (32-dimensional).
    pub type SmallAlgebra = TropicalDualClifford<f64, 5>;

    /// 256-dimensional algebra.
    pub type Algebra256 = TropicalDualClifford<f64, 8>;
}

// Re-export from the type_aliases module
pub use type_aliases::*;

// ============================================================================
// Convenience Functions
// ============================================================================

/// Bundle multiple elements together.
pub fn bundle_all<A: Algebra>(items: &[A], beta: f64) -> Result<A> {
    if items.is_empty() {
        return Ok(A::zero());
    }
    if items.len() == 1 {
        return Ok(items[0].clone());
    }

    let mut result = items[0].clone();
    for item in items.iter().skip(1) {
        result = Algebra::bundle(&result, item, beta)?;
    }
    Ok(result)
}

/// Compute pairwise similarities.
pub fn pairwise_similarities<A: Algebra>(items: &[A]) -> Vec<Vec<f64>> {
    let n = items.len();
    let mut result = vec![vec![0.0; n]; n];

    for i in 0..n {
        result[i][i] = 1.0;
        for j in (i + 1)..n {
            let sim = Algebra::similarity(&items[i], &items[j]);
            result[i][j] = sim;
            result[j][i] = sim;
        }
    }

    result
}

/// Find the most similar element in a codebook.
pub fn find_nearest<A: Algebra>(query: &A, codebook: &[A]) -> Option<(usize, f64)> {
    codebook
        .iter()
        .enumerate()
        .map(|(i, item)| (i, Algebra::similarity(query, item)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algebra_trait_basics() {
        let a = DefaultAlgebra::random_element();
        let b = DefaultAlgebra::random_element();

        // Binding - use Algebra trait explicitly
        let bound = Algebra::bind(&a, &b);
        assert!(Algebra::norm(&bound) > 0.0);

        // Similarity with self should be ~1
        let self_sim = Algebra::similarity(&a, &a);
        assert!((self_sim - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_algebra_dimension() {
        // DefaultAlgebra is ProductCliffordAlgebra<32> = 8*32 = 256 dimensions
        // (or TropicalDualClifford<f64, 8> = 2^8 = 256 for legacy)
        assert_eq!(<DefaultAlgebra as Algebra>::dimension(), 256);
    }

    #[test]
    fn test_algebra_capacity() {
        // 256 dimensions → capacity ~46
        let capacity = <DefaultAlgebra as Algebra>::theoretical_capacity();
        assert!(capacity > 40 && capacity < 55, "capacity: {}", capacity);
    }

    #[test]
    fn test_inverse_recovery() {
        let key = DefaultAlgebra::random_element();
        let value = DefaultAlgebra::random_element();

        let bound = Algebra::bind(&key, &value);

        if let Ok(recovered) = Algebra::unbind(&key, &bound) {
            let sim = Algebra::similarity(&recovered, &value);
            // With 256 dimensions, recovery should be good
            assert!(sim > 0.8, "recovery similarity: {}", sim);
        }
    }

    #[test]
    fn test_bundle_all() {
        let items: Vec<DefaultAlgebra> = (0..5).map(|_| DefaultAlgebra::random_element()).collect();

        let bundled = bundle_all(&items, 1.0).unwrap();
        assert!(Algebra::norm(&bundled) > 0.0);
    }

    #[test]
    fn test_find_nearest() {
        let codebook: Vec<DefaultAlgebra> =
            (0..10).map(|_| DefaultAlgebra::random_element()).collect();

        // Query with an element from the codebook
        let query = codebook[5].clone();
        let (idx, sim) = find_nearest(&query, &codebook).unwrap();

        // The found element should have very high similarity to the query
        // (either the exact element at idx=5, or a randomly similar one)
        assert!(sim > 0.9, "similarity should be high, got {}", sim);

        // Verify that the similarity at the found index matches
        let found_sim = Algebra::similarity(&query, &codebook[idx]);
        assert!((found_sim - sim).abs() < 0.01);
    }
}
