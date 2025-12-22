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

    /// Project to a specific grade.
    fn grade_project(&self, grade: usize) -> Result<Self>
    where
        Self: Sized;

    /// Get the magnitude of a specific grade component.
    fn grade_magnitude(&self, grade: usize) -> T;

    /// Check if the element is predominantly of a specific grade.
    fn is_grade(&self, grade: usize, threshold: T) -> bool;

    /// Normalize to unit magnitude.
    fn normalize(&self) -> Result<Self>
    where
        Self: Sized;

    /// Check if invertible (magnitude above threshold).
    fn is_invertible(&self, epsilon: T) -> bool;

    /// Compute the reversal (grade involution).
    fn reverse(&self) -> Self
    where
        Self: Sized;

    /// Compute the Clifford conjugate.
    fn conjugate(&self) -> Self
    where
        Self: Sized;

    /// Compute sandwich product: self ⊛ x ⊛ self⁻¹.
    ///
    /// This is the fundamental operation for rotations and reflections.
    fn sandwich(&self, x: &Self) -> Result<Self>
    where
        Self: Sized;
}

impl<T: MinuetFloat, const DIM: usize> BindingAlgebra<T, DIM> for TropicalDualClifford<T, DIM> {
    #[cfg_attr(feature = "contracts", ensures(result.is_ok() ==> result.as_ref().unwrap().magnitude() > T::zero()))]
    fn verified_bind(&self, other: &Self) -> Result<Self> {
        let result = self.bind(other);
        let mag = result.magnitude();

        if mag.is_nan() || mag.is_infinite() {
            return Err(MinuetError::AlgebraicConstraint {
                constraint: "bind result has invalid magnitude".into(),
                value: mag.to_f64().unwrap_or(f64::NAN),
            });
        }

        Ok(result)
    }

    fn verified_unbind(&self, other: &Self) -> Result<Self> {
        let result = self.unbind(other);
        let mag = result.magnitude();

        if mag.is_nan() || mag.is_infinite() {
            return Err(MinuetError::AlgebraicConstraint {
                constraint: "unbind result has invalid magnitude".into(),
                value: mag.to_f64().unwrap_or(f64::NAN),
            });
        }

        Ok(result)
    }

    fn grade_project(&self, grade: usize) -> Result<Self> {
        let max_grade = DIM; // In Cl(n), max grade is n
        if grade > max_grade {
            return Err(MinuetError::InvalidGrade { grade, dim: DIM });
        }

        Ok(self.project_grade(grade))
    }

    fn grade_magnitude(&self, grade: usize) -> T {
        self.grade_norm(grade)
    }

    fn is_grade(&self, grade: usize, threshold: T) -> bool {
        let target_mag = self.grade_magnitude(grade);
        let total_mag = self.magnitude();

        if total_mag < T::MIN_POSITIVE {
            return false;
        }

        (target_mag / total_mag) >= threshold
    }

    fn normalize(&self) -> Result<Self> {
        let mag = self.magnitude();

        if mag < T::MIN_POSITIVE {
            return Err(MinuetError::SingularInverse {
                magnitude: mag.to_f64().unwrap_or(0.0),
                epsilon: T::MIN_POSITIVE.to_f64().unwrap_or(1e-300),
            });
        }

        Ok(self.scale(T::one() / mag))
    }

    fn is_invertible(&self, epsilon: T) -> bool {
        self.magnitude() >= epsilon
    }

    fn reverse(&self) -> Self {
        self.reversion()
    }

    fn conjugate(&self) -> Self {
        self.clifford_conjugate()
    }

    fn sandwich(&self, x: &Self) -> Result<Self> {
        if !self.is_invertible(T::MIN_POSITIVE) {
            return Err(MinuetError::SingularInverse {
                magnitude: self.magnitude().to_f64().unwrap_or(0.0),
                epsilon: T::MIN_POSITIVE.to_f64().unwrap_or(1e-300),
            });
        }

        let inv = self.binding_inverse();
        Ok(self.bind(x).bind(&inv))
    }
}

/// Grade projection utilities.
pub struct GradeProjection<T: MinuetFloat, const DIM: usize> {
    _phantom: PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> GradeProjection<T, DIM> {
    /// Extract the scalar (grade-0) component.
    #[must_use]
    pub fn scalar(elem: &TropicalDualClifford<T, DIM>) -> T {
        elem.scalar_part()
    }

    /// Extract the vector (grade-1) components.
    #[must_use]
    pub fn vector(elem: &TropicalDualClifford<T, DIM>) -> TropicalDualClifford<T, DIM> {
        elem.project_grade(1)
    }

    /// Extract the bivector (grade-2) components.
    #[must_use]
    pub fn bivector(elem: &TropicalDualClifford<T, DIM>) -> TropicalDualClifford<T, DIM> {
        elem.project_grade(2)
    }

    /// Extract the pseudoscalar (grade-n) component.
    #[must_use]
    pub fn pseudoscalar(elem: &TropicalDualClifford<T, DIM>) -> TropicalDualClifford<T, DIM> {
        elem.project_grade(DIM)
    }

    /// Decompose into even and odd parts.
    #[must_use]
    pub fn even_odd(
        elem: &TropicalDualClifford<T, DIM>,
    ) -> (TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>) {
        let even = elem.even_part();
        let odd = elem.odd_part();
        (even, odd)
    }

    /// Check if element is even (all odd grades zero).
    #[must_use]
    pub fn is_even(elem: &TropicalDualClifford<T, DIM>, threshold: T) -> bool {
        elem.odd_part().magnitude() < threshold
    }

    /// Check if element is odd (all even grades zero).
    #[must_use]
    pub fn is_odd(elem: &TropicalDualClifford<T, DIM>, threshold: T) -> bool {
        elem.even_part().magnitude() < threshold
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
        epsilon: T,
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
                magnitude: self.inner.magnitude().to_f64().unwrap_or(0.0),
                epsilon: epsilon.to_f64().unwrap_or(1e-10),
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
        let normalized = self.inner.normalize()?;
        Ok(VerifiedElement {
            inner: normalized,
            _inv: PhantomData,
            _norm: PhantomData,
            _grade: PhantomData,
        })
    }

    /// Compute the inverse (only available for verified invertible elements).
    #[must_use]
    pub fn inverse(&self) -> TropicalDualClifford<T, DIM> {
        self.inner.binding_inverse()
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
        let a: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let b: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let result = a.verified_bind(&b).unwrap();
        assert!(!result.magnitude().is_nan());
    }

    #[test]
    fn grade_projection() {
        let elem: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        // Project to various grades
        for grade in 0..=8 {
            let proj = elem.grade_project(grade).unwrap();
            assert!(proj.magnitude() >= 0.0);
        }
    }

    #[test]
    fn normalization() {
        let elem: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let normalized = elem.normalize().unwrap();

        let mag = normalized.magnitude();
        assert!((mag - 1.0).abs() < 1e-10);
    }

    #[test]
    fn sandwich_product() {
        let rotor: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor();
        let vector: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        let result = rotor.sandwich(&vector).unwrap();
        // Sandwich should preserve magnitude
        let orig_mag = vector.magnitude();
        let result_mag = result.magnitude();
        assert!((orig_mag - result_mag).abs() / orig_mag < 0.1);
    }

    #[test]
    fn verified_element_workflow() {
        let tdc: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let elem = VerifiedElement::new(tdc);
        let invertible = elem.verify_invertible(1e-10).unwrap();
        let normalized = invertible.normalize().unwrap();

        let mag = normalized.inner().magnitude();
        assert!((mag - 1.0).abs() < 1e-10);
    }
}
