//! Binding algebra operations on TDC elements.
//!
//! This module wraps amari-fusion's binding operations with additional
//! type safety, contracts, and grade-aware operations.

use std::marker::PhantomData;

use amari_fusion::{holographic::Bindable, TropicalDualClifford};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{markers::*, MinuetError, Result};
use crate::precision::MinuetFloat;

/// Extended binding algebra with grade projections and verified operations.
///
/// This trait extends the base `Bindable` trait with additional operations
/// for grade-aware computation and algebraic verification.
pub trait BindingAlgebra<T: MinuetFloat, const DIM: usize>: Bindable {
    /// Bind with verification that the result is well-formed.
    ///
    /// # Errors
    ///
    /// Returns error if the resulting element has numerical issues.
    fn verified_bind(&self, other: &Self) -> Result<Self>
    where
        Self: Sized;

    /// Unbind with verification.
    fn verified_unbind(&self, other: &Self) -> Result<Self>
    where
        Self: Sized;

    /// Get the magnitude of a specific grade component.
    ///
    /// Uses the grade spectrum from the underlying representation.
    fn grade_magnitude(&self, grade: usize) -> f64;

    /// Get the dominant grade (grade with largest magnitude).
    fn dominant_grade(&self) -> usize;

    /// Check if the element is predominantly of a specific grade.
    fn is_grade(&self, grade: usize, threshold: f64) -> bool;

    /// Normalize to unit magnitude.
    fn verified_normalize(&self) -> Result<Self>
    where
        Self: Sized;

    /// Check if invertible (magnitude above threshold).
    fn is_invertible(&self, epsilon: f64) -> bool;

    /// Compute sandwich product: self ⊛ x ⊛ self⁻¹.
    ///
    /// This is the fundamental operation for rotations and reflections.
    fn sandwich(&self, x: &Self) -> Result<Self>
    where
        Self: Sized;

    /// Get the Clifford norm (magnitude).
    fn magnitude(&self) -> f64;
}

impl<T: MinuetFloat, const DIM: usize> BindingAlgebra<T, DIM> for TropicalDualClifford<T, DIM> {
    fn verified_bind(&self, other: &Self) -> Result<Self> {
        let result = self.bind(other);
        let mag = result.norm();

        if mag.is_nan() || mag.is_infinite() {
            return Err(MinuetError::AlgebraicConstraint {
                constraint: "bind result has invalid magnitude".into(),
                value: mag,
            });
        }

        Ok(result)
    }

    fn verified_unbind(&self, other: &Self) -> Result<Self> {
        let result = self.unbind(other);
        let mag = result.norm();

        if mag.is_nan() || mag.is_infinite() {
            return Err(MinuetError::AlgebraicConstraint {
                constraint: "unbind result has invalid magnitude".into(),
                value: mag,
            });
        }

        Ok(result)
    }

    fn grade_magnitude(&self, grade: usize) -> f64 {
        let spectrum = self.grade_spectrum();
        spectrum.get(grade).copied().unwrap_or(f64::NEG_INFINITY)
    }

    fn dominant_grade(&self) -> usize {
        self.dominant_grade()
    }

    fn is_grade(&self, grade: usize, threshold: f64) -> bool {
        let spectrum = self.grade_spectrum();
        let total: f64 = spectrum.iter().map(|x| x.exp()).sum();

        if total < 1e-10 {
            return false;
        }

        let grade_mag = spectrum
            .get(grade)
            .copied()
            .unwrap_or(f64::NEG_INFINITY)
            .exp();
        (grade_mag / total) >= threshold
    }

    fn verified_normalize(&self) -> Result<Self> {
        let mag = self.norm();

        if mag < 1e-10 {
            return Err(MinuetError::SingularInverse {
                magnitude: mag,
                epsilon: 1e-10,
            });
        }

        Ok(self.normalize())
    }

    fn is_invertible(&self, epsilon: f64) -> bool {
        self.norm() >= epsilon
    }

    fn sandwich(&self, x: &Self) -> Result<Self> {
        if !self.is_invertible(1e-10) {
            return Err(MinuetError::SingularInverse {
                magnitude: self.norm(),
                epsilon: 1e-10,
            });
        }

        match self.binding_inverse() {
            Some(inv) => Ok(self.bind(x).bind(&inv)),
            None => Err(MinuetError::SingularInverse {
                magnitude: self.norm(),
                epsilon: 1e-10,
            }),
        }
    }

    fn magnitude(&self) -> f64 {
        self.norm()
    }
}

/// Grade projection utilities.
///
/// Note: Full grade projection requires access to the underlying Clifford
/// algebra. These utilities work with the grade spectrum available from TDC.
pub struct GradeProjection<T: MinuetFloat, const DIM: usize> {
    _phantom: PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> GradeProjection<T, DIM> {
    /// Get the grade spectrum (log-magnitudes of all grades).
    #[must_use]
    pub fn spectrum(elem: &TropicalDualClifford<T, DIM>) -> Vec<f64> {
        elem.grade_spectrum()
    }

    /// Get the dominant grade.
    #[must_use]
    pub fn dominant(elem: &TropicalDualClifford<T, DIM>) -> usize {
        elem.dominant_grade()
    }

    /// Check if element is predominantly scalar (grade-0).
    #[must_use]
    pub fn is_scalar(elem: &TropicalDualClifford<T, DIM>, threshold: f64) -> bool {
        elem.is_grade(0, threshold)
    }

    /// Check if element is predominantly vector (grade-1).
    #[must_use]
    pub fn is_vector(elem: &TropicalDualClifford<T, DIM>, threshold: f64) -> bool {
        elem.is_grade(1, threshold)
    }

    /// Check if element is predominantly bivector (grade-2).
    #[must_use]
    pub fn is_bivector(elem: &TropicalDualClifford<T, DIM>, threshold: f64) -> bool {
        elem.is_grade(2, threshold)
    }
}

/// Type-safe wrapper for verified binding operations.
///
/// Uses phantom types to track algebraic properties at compile time.
#[derive(Debug, Clone)]
pub struct VerifiedElement<T: MinuetFloat, const DIM: usize, Inv, Norm, Grade> {
    inner: TropicalDualClifford<T, DIM>,
    _inv: PhantomData<Inv>,
    _norm: PhantomData<Norm>,
    _grade: PhantomData<Grade>,
}

impl<T: MinuetFloat, const DIM: usize>
    VerifiedElement<T, DIM, MaybeInvertible, Unnormalized, MixedGrade>
{
    /// Create a new unverified element.
    #[must_use]
    pub fn new(inner: TropicalDualClifford<T, DIM>) -> Self {
        Self {
            inner,
            _inv: PhantomData,
            _norm: PhantomData,
            _grade: PhantomData,
        }
    }

    /// Verify invertibility.
    pub fn verify_invertible(
        self,
        epsilon: f64,
    ) -> Result<VerifiedElement<T, DIM, Invertible, Unnormalized, MixedGrade>> {
        if self.inner.is_invertible(epsilon) {
            Ok(VerifiedElement {
                inner: self.inner,
                _inv: PhantomData,
                _norm: PhantomData,
                _grade: PhantomData,
            })
        } else {
            Err(MinuetError::SingularInverse {
                magnitude: self.inner.norm(),
                epsilon,
            })
        }
    }

    /// Get the inner value.
    #[must_use]
    pub fn inner(&self) -> &TropicalDualClifford<T, DIM> {
        &self.inner
    }

    /// Consume and return inner value.
    #[must_use]
    pub fn into_inner(self) -> TropicalDualClifford<T, DIM> {
        self.inner
    }
}

impl<T: MinuetFloat, const DIM: usize, Grade>
    VerifiedElement<T, DIM, Invertible, Unnormalized, Grade>
{
    /// Normalize the element (only available for invertible elements).
    pub fn normalize(self) -> Result<VerifiedElement<T, DIM, Invertible, Normalized, Grade>> {
        let normalized = self.inner.verified_normalize()?;
        Ok(VerifiedElement {
            inner: normalized,
            _inv: PhantomData,
            _norm: PhantomData,
            _grade: PhantomData,
        })
    }

    /// Compute the inverse (only available for verified invertible elements).
    #[must_use]
    pub fn inverse(&self) -> Option<TropicalDualClifford<T, DIM>> {
        self.inner.binding_inverse()
    }

    /// Get the inner value.
    #[must_use]
    pub fn inner(&self) -> &TropicalDualClifford<T, DIM> {
        &self.inner
    }
}

impl<T: MinuetFloat, const DIM: usize, Inv, Norm, Grade> VerifiedElement<T, DIM, Inv, Norm, Grade> {
    /// Bind with another element.
    pub fn bind<Inv2, Norm2, Grade2>(
        &self,
        other: &VerifiedElement<T, DIM, Inv2, Norm2, Grade2>,
    ) -> VerifiedElement<T, DIM, MaybeInvertible, Unnormalized, MixedGrade> {
        VerifiedElement::new(self.inner.bind(&other.inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_preserves_validity() {
        let a: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        let b: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        let result = a.verified_bind(&b).unwrap();
        assert!(!result.norm().is_nan());
    }

    #[test]
    fn grade_spectrum() {
        let elem: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        let spectrum = GradeProjection::<f64, 8>::spectrum(&elem);
        assert!(!spectrum.is_empty());

        let dominant = GradeProjection::<f64, 8>::dominant(&elem);
        assert!(dominant <= 8);
    }

    #[test]
    fn normalization() {
        let elem: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        let normalized = elem.verified_normalize().unwrap();

        let mag = normalized.norm();
        assert!((mag - 1.0).abs() < 1e-10);
    }

    #[test]
    fn sandwich_product() {
        let rotor: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let vector: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        let result = rotor.sandwich(&vector).unwrap();
        // Sandwich should preserve magnitude (approximately)
        let orig_mag = vector.norm();
        let result_mag = result.norm();
        assert!((orig_mag - result_mag).abs() / orig_mag.max(1e-10) < 0.5);
    }

    #[test]
    fn verified_element_workflow() {
        let tdc: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        let elem = VerifiedElement::new(tdc);
        let invertible = elem.verify_invertible(1e-10).unwrap();
        let normalized = invertible.normalize().unwrap();

        let mag = normalized.inner.norm();
        assert!((mag - 1.0).abs() < 1e-10);
    }
}
