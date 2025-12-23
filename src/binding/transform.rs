//! Reified transformations between representations.
//!
//! Transformations are extracted from example pairs and can be applied to new inputs.
//! The Clifford structure means geometric transformations (rotations, reflections)
//! are represented exactly.

use std::marker::PhantomData;

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{markers::*, MinuetError, Result};
use crate::precision::MinuetFloat;

/// Metadata about a transformation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransformMetadata {
    /// Human-readable description.
    pub description: Option<String>,

    /// Source examples used to derive this transform.
    pub source_examples: Vec<(String, String)>,

    /// Estimated fidelity (how well does apply(extract_source) == extract_target).
    pub fidelity: f64,

    /// Whether this is a simple versor (rotation/reflection).
    pub is_versor: bool,
}

/// A reified transformation between representations.
///
/// Transformations are extracted from example pairs and applied to new inputs.
/// The Clifford structure means geometric transformations (rotations, reflections)
/// are represented exactly as versors.
///
/// # Type Parameters
///
/// * `T` - Numeric type
/// * `DIM` - Representation dimensionality
/// * `Inv` - Invertibility marker
#[derive(Debug, Clone)]
pub struct Transform<T: MinuetFloat, const DIM: usize, Inv = MaybeInvertible> {
    /// The transformation as a TDC element (typically a versor).
    repr: TropicalDualClifford<T, DIM>,

    /// Metadata about the transformation.
    metadata: TransformMetadata,

    /// Invertibility phantom.
    _inv: PhantomData<Inv>,
}

impl<T: MinuetFloat, const DIM: usize> Transform<T, DIM, MaybeInvertible> {
    /// Extract a transformation from a single before/after pair.
    ///
    /// The transformation is: `transform = after ⊛ before⁻¹`
    ///
    /// # Arguments
    ///
    /// * `before` - The source element
    /// * `after` - The target element
    ///
    /// # Returns
    ///
    /// The extracted transformation, or error if before is not invertible.
    pub fn extract(
        before: &TropicalDualClifford<T, DIM>,
        after: &TropicalDualClifford<T, DIM>,
    ) -> Result<Self> {
        let before_mag = before.norm();

        if before_mag < 1e-10 {
            return Err(MinuetError::TransformExtraction(
                "before element is not invertible (near-zero magnitude)".into(),
            ));
        }

        let before_inv = match before.binding_inverse() {
            Some(inv) => inv,
            None => {
                return Err(MinuetError::TransformExtraction(
                    "before element has no inverse".into(),
                ))
            }
        };

        let repr = after.bind(&before_inv);

        // Compute fidelity: how close is apply(before) to after?
        let reconstructed = repr.bind(before);
        let fidelity = reconstructed.similarity(after);

        Ok(Self {
            repr,
            metadata: TransformMetadata {
                description: None,
                source_examples: Vec::new(),
                fidelity,
                is_versor: false,
            },
            _inv: PhantomData,
        })
    }

    /// Extract a transformation from multiple example pairs (averaged).
    ///
    /// Uses weighted averaging based on the magnitude of individual transforms.
    ///
    /// # Arguments
    ///
    /// * `pairs` - (before, after) pairs defining the transformation
    ///
    /// # Returns
    ///
    /// The averaged transformation, or error if no valid transforms could be extracted.
    pub fn extract_from_examples(
        pairs: &[(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)],
    ) -> Result<Self> {
        if pairs.is_empty() {
            return Err(MinuetError::TransformExtraction(
                "no examples provided".into(),
            ));
        }

        // Extract individual transforms and bundle them
        let mut combined: TropicalDualClifford<T, DIM> = TropicalDualClifford::new();
        let mut valid_count = 0;
        let mut total_fidelity = 0.0;

        for (before, after) in pairs {
            if let Ok(transform) = Self::extract(before, after) {
                combined = combined.bundle(&transform.repr, 1.0);
                total_fidelity += transform.metadata.fidelity;
                valid_count += 1;
            }
        }

        if valid_count == 0 {
            return Err(MinuetError::TransformExtraction(
                "no valid transforms could be extracted from examples".into(),
            ));
        }

        let avg_fidelity = total_fidelity / valid_count as f64;

        Ok(Self {
            repr: combined,
            metadata: TransformMetadata {
                description: None,
                source_examples: Vec::new(),
                fidelity: avg_fidelity,
                is_versor: false,
            },
            _inv: PhantomData,
        })
    }

    /// Create a transform from a raw representation.
    #[must_use]
    pub fn from_repr(repr: TropicalDualClifford<T, DIM>) -> Self {
        Self {
            repr,
            metadata: TransformMetadata::default(),
            _inv: PhantomData,
        }
    }

    /// Verify that this transform is invertible.
    pub fn verify_invertible(self, epsilon: f64) -> Result<Transform<T, DIM, Invertible>> {
        let mag = self.repr.norm();

        if mag < epsilon {
            return Err(MinuetError::SingularInverse {
                message: format!("magnitude {mag:.6} below epsilon {epsilon:.6}"),
            });
        }

        Ok(Transform {
            repr: self.repr,
            metadata: self.metadata,
            _inv: PhantomData,
        })
    }
}

impl<T: MinuetFloat, const DIM: usize, Inv> Transform<T, DIM, Inv> {
    /// Apply this transformation to an input.
    ///
    /// Computes: `result = transform ⊛ input`
    #[must_use]
    pub fn apply(&self, input: &TropicalDualClifford<T, DIM>) -> TropicalDualClifford<T, DIM> {
        self.repr.bind(input)
    }

    /// Compose with another transformation.
    ///
    /// The result represents: first self, then other.
    ///
    /// Computes: `result = other ⊛ self`
    #[must_use]
    pub fn compose<Inv2>(
        &self,
        other: &Transform<T, DIM, Inv2>,
    ) -> Transform<T, DIM, MaybeInvertible> {
        let composed = other.repr.bind(&self.repr);

        Transform {
            repr: composed,
            metadata: TransformMetadata {
                description: Some("Composed transformation".into()),
                source_examples: Vec::new(),
                fidelity: self.metadata.fidelity * other.metadata.fidelity,
                is_versor: self.metadata.is_versor && other.metadata.is_versor,
            },
            _inv: PhantomData,
        }
    }

    /// Get the underlying representation.
    #[must_use]
    pub fn repr(&self) -> &TropicalDualClifford<T, DIM> {
        &self.repr
    }

    /// Get transformation metadata.
    #[must_use]
    pub fn metadata(&self) -> &TransformMetadata {
        &self.metadata
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = Some(description.into());
        self
    }

    /// Add source example names.
    pub fn with_example(mut self, before: impl Into<String>, after: impl Into<String>) -> Self {
        self.metadata
            .source_examples
            .push((before.into(), after.into()));
        self
    }

    /// Compute the magnitude of the transformation.
    #[must_use]
    pub fn magnitude(&self) -> f64 {
        self.repr.norm()
    }

    /// Check if this is close to the identity transformation.
    #[must_use]
    pub fn is_identity(&self, threshold: f64) -> bool {
        let identity = TropicalDualClifford::<T, DIM>::binding_identity();
        self.repr.similarity(&identity) > (1.0 - threshold)
    }
}

impl<T: MinuetFloat, const DIM: usize> Transform<T, DIM, Invertible> {
    /// Apply the inverse transformation.
    ///
    /// Computes: `result = transform⁻¹ ⊛ input`
    #[must_use]
    pub fn apply_inverse(
        &self,
        input: &TropicalDualClifford<T, DIM>,
    ) -> TropicalDualClifford<T, DIM> {
        // Safe to unwrap since we verified invertibility
        let inv = self.repr.binding_inverse().expect("verified invertible");
        inv.bind(input)
    }

    /// Compute the inverse transformation.
    #[must_use]
    pub fn inverse(&self) -> Transform<T, DIM, Invertible> {
        // Safe to unwrap since we verified invertibility
        let inv_repr = self.repr.binding_inverse().expect("verified invertible");

        Transform {
            repr: inv_repr,
            metadata: TransformMetadata {
                description: self
                    .metadata
                    .description
                    .as_ref()
                    .map(|d| format!("Inverse of: {}", d)),
                source_examples: self
                    .metadata
                    .source_examples
                    .iter()
                    .map(|(a, b)| (b.clone(), a.clone()))
                    .collect(),
                fidelity: self.metadata.fidelity,
                is_versor: self.metadata.is_versor,
            },
            _inv: PhantomData,
        }
    }

    /// Interpolate between identity and this transformation.
    ///
    /// t=0: identity, t=1: full transformation
    ///
    /// Uses geometric interpolation (SLERP-like for versors).
    #[must_use]
    pub fn interpolate(&self, t: T) -> Transform<T, DIM, MaybeInvertible> {
        let identity = TropicalDualClifford::<T, DIM>::binding_identity();

        // Linear interpolation in TDC space (not true SLERP, but sufficient)
        let one_minus_t = T::one() - t;
        let interpolated = identity.scale(one_minus_t).add(&self.repr.scale(t));

        Transform {
            repr: interpolated,
            metadata: TransformMetadata {
                description: Some(format!("Interpolated (t={:.2})", t.to_f64().unwrap_or(0.0))),
                source_examples: Vec::new(),
                fidelity: self.metadata.fidelity,
                is_versor: false, // Interpolation may not preserve versor property
            },
            _inv: PhantomData,
        }
    }
}

/// Builder for constructing transforms with specific properties.
#[derive(Debug)]
pub struct TransformBuilder<T: MinuetFloat, const DIM: usize> {
    examples: Vec<(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)>,
    description: Option<String>,
}

impl<T: MinuetFloat, const DIM: usize> TransformBuilder<T, DIM> {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            examples: Vec::new(),
            description: None,
        }
    }

    /// Add an example pair.
    #[must_use]
    pub fn example(
        mut self,
        before: TropicalDualClifford<T, DIM>,
        after: TropicalDualClifford<T, DIM>,
    ) -> Self {
        self.examples.push((before, after));
        self
    }

    /// Set description.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Build the transform.
    pub fn build(self) -> Result<Transform<T, DIM, MaybeInvertible>> {
        let mut transform = Transform::extract_from_examples(&self.examples)?;

        if let Some(desc) = self.description {
            transform.metadata.description = Some(desc);
        }

        Ok(transform)
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for TransformBuilder<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_and_apply() {
        // Use random_versor which actually generates random elements
        let before: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let after: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);

        let transform = Transform::extract(&before, &after).unwrap();

        // Apply to before should give something similar to after
        let result = transform.apply(&before);
        let sim = result.similarity(&after);

        // Fidelity should be reasonable
        assert!(sim > 0.5);
    }

    #[test]
    fn identity_transform() {
        // Use random_versor which actually generates random elements
        let elem: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);

        // Extract identity: before == after
        let transform = Transform::extract(&elem, &elem).unwrap();

        assert!(transform.is_identity(0.1));
    }

    #[test]
    fn transform_composition() {
        // Use random_versor which actually generates random elements
        let a: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let b: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let c: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);

        let t1 = Transform::extract(&a, &b).unwrap();
        let t2 = Transform::extract(&b, &c).unwrap();

        let composed = t1.compose(&t2);

        // composed.apply(a) should be similar to c
        let result = composed.apply(&a);
        let direct = Transform::extract(&a, &c).unwrap().apply(&a);

        // Both should give similar results
        assert!(result.similarity(&direct) > 0.5);
    }

    #[test]
    fn invertible_transform() {
        // Use random_versor which actually generates random elements
        let before: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let after: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);

        let transform = Transform::extract(&before, &after)
            .unwrap()
            .verify_invertible(1e-10)
            .unwrap();

        // Apply then inverse should return to original
        let forward = transform.apply(&before);
        let back = transform.apply_inverse(&forward);

        assert!(back.similarity(&before) > 0.8);
    }

    #[test]
    fn builder_pattern() {
        // Use random_versor which actually generates random elements
        let a: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let b: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let c: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);
        let d: TropicalDualClifford<f64, 8> = TropicalDualClifford::random_versor(2);

        let transform = TransformBuilder::new()
            .example(a, b)
            .example(c, d)
            .description("Test transform")
            .build()
            .unwrap();

        assert!(transform.metadata().description.is_some());
    }
}
