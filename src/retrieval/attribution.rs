// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0
//! Attribution and provenance tracking for holographic retrieval.
//!
//! Attribution quantifies how much each stored binding contributes to a
//! retrieval result. It is substrate-agnostic: it works over any
//! [`BindingAlgebra`](amari_holographic::BindingAlgebra) (Clifford, FHRR, MAP,
//! …), so a noisy microwave backend (Kagome) gets the same provenance signal
//! as the reference optical algebra.
//!
//! Restored in the Kagome-readiness sprint (WS 3) from the pre-v0.3.0
//! codebase. The original was hard-wired to `TropicalDualClifford`; this port
//! re-expresses it generically over `A: BindingAlgebra` (per handoff §2,
//! decision B). The core signal is similarity-based: for each stored binding
//! `key ⊛ value`, we unbind the query and measure similarity to the result.

use std::cmp::Ordering;
use std::collections::HashMap;

use amari_holographic::BindingAlgebra;

use crate::error::{MinuetError, Result};

/// Inner product on a [`BindingAlgebra`], recovered from the trait's
/// `similarity` (cosine) and `norm` (Euclidean) primitives:
/// `⟨a, b⟩ = similarity(a, b) · ‖a‖ · ‖b‖`.
///
/// Used by [`Attribution::compute_gradient`] for the projection-based exact
/// attribution. Zero when either operand has zero norm.
fn inner_product<A: BindingAlgebra>(a: &A, b: &A) -> f64 {
    a.similarity(b) * a.norm() * b.norm()
}

/// Attribution information for a retrieval result.
///
/// Produced by [`Attribution::compute`]. Contributions are normalized so that
/// the tracked mass sums to ~1.0 (when any positive contributions exist).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttributionResult {
    /// Store ID to contribution mapping.
    pub contributions: HashMap<u64, f64>,

    /// Top contributors sorted by descending contribution.
    pub top_contributors: Vec<(u64, f64)>,

    /// Total attribution mass (should sum to ~1.0 for normalized results).
    pub total_mass: f64,

    /// Whether attribution is complete or approximate.
    pub is_approximate: bool,
}

impl AttributionResult {
    /// Get the contribution of a specific store operation.
    #[must_use]
    pub fn contribution(&self, store_id: u64) -> Option<f64> {
        self.contributions.get(&store_id).copied()
    }

    /// Get the top `n` contributors (descending).
    #[must_use]
    pub fn top_n(&self, n: usize) -> &[(u64, f64)] {
        &self.top_contributors[..n.min(self.top_contributors.len())]
    }

    /// Check if a specific store was a significant contributor (at or above
    /// `threshold`).
    #[must_use]
    pub fn is_significant(&self, store_id: u64, threshold: f64) -> bool {
        self.contributions
            .get(&store_id)
            .is_some_and(|&c| c >= threshold)
    }

    /// Get store IDs whose contribution is at or above `threshold`.
    #[must_use]
    pub fn above_threshold(&self, threshold: f64) -> Vec<u64> {
        self.contributions
            .iter()
            .filter(|(_, &v)| v >= threshold)
            .map(|(&k, _)| k)
            .collect()
    }
}

/// Attribution calculator for holographic memory, generic over the binding
/// algebra.
///
/// Register stored bindings via [`register`](Self::register), then call
/// [`compute`](Self::compute) with the query and retrieval result to obtain an
/// [`AttributionResult`].
///
/// # Examples
///
/// ```
/// use amari_holographic::{BindingAlgebra, ProductCliffordAlgebra};
/// use minuet::retrieval::Attribution;
/// type Algebra = ProductCliffordAlgebra<8>;
///
/// let mut attr = Attribution::<Algebra>::new();
///
/// // Bind key1 ~ value1, register the trace.
/// let key1 = Algebra::random_versor(2);
/// let val1 = Algebra::random_versor(2);
/// attr.register(1, key1.bind(&val1));
///
/// // Querying with key1 should attribute the result to store id 1.
/// let attribution = attr.compute(&key1, &val1)?;
/// assert!(attribution.contribution(1).is_some());
/// # Ok::<(), minuet::error::MinuetError>(())
/// ```
pub struct Attribution<A: BindingAlgebra> {
    /// Stored bindings with their IDs.
    bindings: Vec<(u64, A)>,

    /// Whether to report approximate (fast) or exact attribution.
    approximate: bool,

    /// Contribution threshold for filtering.
    threshold: f64,

    /// Maximum number of attributions to return.
    max_attributions: usize,
}

impl<A: BindingAlgebra> Attribution<A> {
    /// Create a new attribution calculator (approximate mode, threshold 0.01,
    /// at most 100 attributions).
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            approximate: true,
            threshold: 0.01,
            max_attributions: 100,
        }
    }

    /// Switch to exact (slower but more precise) mode.
    #[must_use]
    pub fn exact(mut self) -> Self {
        self.approximate = false;
        self
    }

    /// Set the contribution threshold.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the maximum number of attributions returned.
    #[must_use]
    pub fn with_max(mut self, max: usize) -> Self {
        self.max_attributions = max;
        self
    }

    /// Register a stored binding for attribution tracking.
    pub fn register(&mut self, store_id: u64, binding: A) {
        self.bindings.push((store_id, binding));
    }

    /// Clear all registered bindings.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    /// Compute attribution for a retrieval result.
    ///
    /// For each registered binding `key ⊛ value`, the query is unbound
    /// (`query.inverse() ⊛ binding`) and its similarity to `result` is the raw
    /// contribution. Contributions above the threshold are normalized so they
    /// sum to ~1.0.
    ///
    /// # Errors
    ///
    /// Returns [`MinuetError::Algebra`] if `query` cannot be unbound (e.g. it
    /// is non-invertible); every binding shares `query` as the unbinding key,
    /// so failure is uniform rather than per-binding.
    pub fn compute(&self, query: &A, result: &A) -> Result<AttributionResult> {
        if self.bindings.is_empty() {
            return Ok(AttributionResult {
                contributions: HashMap::new(),
                top_contributors: Vec::new(),
                total_mass: 0.0,
                is_approximate: self.approximate,
            });
        }

        // Compute the contribution of each binding. `query.unbind(binding)`
        // computes `query.inverse() ⊛ binding`; the unbinding succeeds or fails
        // uniformly across bindings (it depends on `query`'s invertibility).
        let mut contributions = HashMap::new();
        let mut total_mass = 0.0;

        for (store_id, binding) in &self.bindings {
            let unbound = query.unbind(binding).map_err(MinuetError::algebra)?;
            let sim = unbound.similarity(result);

            // Only track positive contributions above threshold.
            if sim > self.threshold {
                contributions.insert(*store_id, sim);
                total_mass += sim;
            }
        }

        // Normalize contributions if we have any.
        if total_mass > 0.0 {
            for v in contributions.values_mut() {
                *v /= total_mass;
            }
        }

        // Sort by descending contribution.
        let mut top_contributors: Vec<(u64, f64)> =
            contributions.iter().map(|(&k, &v)| (k, v)).collect();
        top_contributors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        top_contributors.truncate(self.max_attributions);

        Ok(AttributionResult {
            contributions,
            top_contributors,
            total_mass: 1.0, // Normalized
            is_approximate: self.approximate,
        })
    }

    /// Compute attribution via forward-mode dual (gradient) propagation over
    /// the binding algebra.
    ///
    /// This is the *exact* attribution path, distinct from the similarity
    /// heuristic [`compute`](Self::compute). For each registered binding
    /// `bᵢ = keyᵢ ⊛ valueᵢ`, the retrieved contribution is
    /// `rᵢ = query⁻¹ ⊛ bᵢ`. These `rᵢ` are the forward-mode Jacobian columns
    /// `∂result/∂(presenceᵢ)` — i.e. how the retrieved superposition responds
    /// to each store's presence, propagated through `bind`/`unbind` via the
    /// geometric product's product rule.
    ///
    /// Attribution is the orthogonal projection of each `rᵢ` onto `result`:
    ///
    /// ```text
    /// attribᵢ = ⟨rᵢ, result⟩ / ⟨result, result⟩
    /// ```
    ///
    /// where `⟨a, b⟩ = similarity(a, b) · ‖a‖ · ‖b‖`. When `result` is the
    /// raw retrieved superposition `Σᵢ rᵢ`, the attributions sum to exactly
    /// `1.0` (the projection of a sum onto itself is exact). For overlapping
    /// / non-orthogonal stores this is strictly more correct than `compute`,
    /// whose cosine heuristic double-counts shared mass.
    ///
    /// # Implementation note (WS 4b)
    ///
    /// The pre-v0.3.0 code intended this via the dual-number component of
    /// `TropicalDualClifford`, but (a) that path was a stub, and (b) audit
    /// of `amari-fusion` 0.23 showed its `TropicalDualClifford` reinitializes
    /// the dual representation inside `bind`/`unbind`/`bundle`, so duals do
    /// *not* propagate through holographic ops there. This implementation
    /// instead performs the equivalent forward-mode propagation directly
    /// over [`BindingAlgebra`] (no `amari-fusion` dependency), which is exact
    /// for the linear binding model. Propagation through the *nonlinear*
    /// resonator cleanup (softmax-weighted projection) is the natural
    /// extension and is left as future work; until then, pass the raw
    /// superposition as `result` for the exactness guarantee.
    ///
    /// # Errors
    ///
    /// Returns [`MinuetError::Algebra`] if `query` cannot be unbound.
    pub fn compute_gradient(&self, query: &A, result: &A) -> Result<AttributionResult> {
        if self.bindings.is_empty() {
            return Ok(AttributionResult {
                contributions: HashMap::new(),
                top_contributors: Vec::new(),
                total_mass: 0.0,
                is_approximate: false,
            });
        }

        let result_norm_sq = {
            let n = result.norm();
            n * n
        };
        // A zero-norm result has no direction to project onto.
        if result_norm_sq <= 0.0 {
            return Ok(AttributionResult {
                contributions: HashMap::new(),
                top_contributors: Vec::new(),
                total_mass: 0.0,
                is_approximate: false,
            });
        }

        // r_i = query⁻¹ ⊛ b_i for each binding (the per-store retrieved
        // contribution; failure is uniform in `query`'s invertibility).
        //
        // Every projection is retained — unlike `compute`, no threshold
        // filtering is applied, because the exactness guarantee
        // (Σ attribᵢ ≈ 1.0 against the raw superposition) requires the signed
        // sum over *all* stores. Callers filter via
        // [`AttributionResult::above_threshold`].
        let mut contributions = HashMap::new();
        let mut total_mass = 0.0;
        for (store_id, binding) in &self.bindings {
            let r_i = query.unbind(binding).map_err(MinuetError::algebra)?;
            let proj = inner_product(&r_i, result) / result_norm_sq;
            contributions.insert(*store_id, proj);
            total_mass += proj;
        }

        // Note: against the raw superposition (result == Σ r_i) the projections
        // already sum to ~1.0, so no renormalization is applied — that would
        // destroy the exactness guarantee. `total_mass` reports the realized
        // sum (≈1.0 against the raw superposition; <1.0 against a cleaned/
        // orthogonal result).
        let mut top_contributors: Vec<(u64, f64)> =
            contributions.iter().map(|(&k, &v)| (k, v)).collect();
        top_contributors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        top_contributors.truncate(self.max_attributions);

        Ok(AttributionResult {
            contributions,
            top_contributors,
            total_mass,
            is_approximate: false,
        })
    }

    /// Get the number of registered bindings.
    #[must_use]
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}

impl<A: BindingAlgebra> Default for Attribution<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for attribution queries.
///
/// Holds a query/result pair plus optional store filtering. Currently a
/// self-contained builder; `Attribution::compute` takes the query and result
/// directly, so this type is most useful for callers that want to assemble a
/// query description before handing its fields to `compute`.
#[derive(Debug)]
pub struct AttributionQuery<A: BindingAlgebra> {
    query: A,
    result: A,
    store_ids: Option<Vec<u64>>,
    threshold: f64,
}

impl<A: BindingAlgebra> AttributionQuery<A> {
    /// Create a new attribution query (threshold 0.01).
    #[must_use]
    pub fn new(query: A, result: A) -> Self {
        Self {
            query,
            result,
            store_ids: None,
            threshold: 0.01,
        }
    }

    /// Restrict attribution to specific store IDs.
    #[must_use]
    pub fn for_stores(mut self, ids: Vec<u64>) -> Self {
        self.store_ids = Some(ids);
        self
    }

    /// Set the contribution threshold.
    #[must_use]
    pub fn threshold(mut self, t: f64) -> Self {
        self.threshold = t;
        self
    }

    /// Borrow the query algebra element.
    #[must_use]
    pub fn query(&self) -> &A {
        &self.query
    }

    /// Borrow the result algebra element.
    #[must_use]
    pub fn result(&self) -> &A {
        &self.result
    }
}

/// Human-readable explanation of a retrieval result.
///
/// Aggregates [`ExplanationFactor`]s (typically derived from an
/// [`AttributionResult`]) into a confidence-scored narrative. Unlike the
/// algebraic types above, this carries no generic parameter and is cheap to
/// serialize for downstream tooling.
#[derive(Debug, Clone)]
pub struct RetrievalExplanation {
    /// The query description.
    pub query_description: String,

    /// The result description.
    pub result_description: String,

    /// Contributing factors.
    pub factors: Vec<ExplanationFactor>,

    /// Confidence in the explanation.
    pub confidence: f64,
}

/// A factor contributing to a retrieval result.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExplanationFactor {
    /// Store ID of the contributing item.
    pub store_id: u64,

    /// Human-readable description (if available).
    pub description: Option<String>,

    /// Contribution weight (0.0 to 1.0).
    pub weight: f64,

    /// How this factor relates to the query.
    pub relation: FactorRelation,
}

/// How a factor relates to the query.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FactorRelation {
    /// Direct match.
    Direct,
    /// Analogical relationship.
    Analogical,
    /// Transformation relationship.
    Transform,
    /// Partial match.
    Partial,
    /// Unknown relationship.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>;

    #[test]
    fn attribution_empty() {
        let attr = Attribution::<TestAlgebra>::new();
        let query = TestAlgebra::random_versor(2);
        let result = TestAlgebra::random_versor(2);

        let attribution = attr.compute(&query, &result).unwrap();
        assert!(attribution.contributions.is_empty());
    }

    #[test]
    fn attribution_with_bindings() {
        let mut attr = Attribution::<TestAlgebra>::new().with_threshold(0.0);

        // Register some bindings.
        let key1 = TestAlgebra::random_versor(2);
        let val1 = TestAlgebra::random_versor(2);
        let binding1 = key1.bind(&val1);
        attr.register(1, binding1);

        let key2 = TestAlgebra::random_versor(2);
        let val2 = TestAlgebra::random_versor(2);
        let binding2 = key2.bind(&val2);
        attr.register(2, binding2);

        // Query with key1; result is val1.
        let result = val1.clone();
        let attribution = attr.compute(&key1, &result).unwrap();

        // Should have some contributions.
        assert!(!attribution.contributions.is_empty());
    }

    #[test]
    fn attribution_filters_to_matched_binding() {
        // The binding whose value matches the result should dominate; the
        // orthogonal binding should contribute little-to-nothing.
        let mut attr = Attribution::<TestAlgebra>::new().with_threshold(0.0);

        let key1 = TestAlgebra::random_versor(2);
        let val1 = TestAlgebra::random_versor(2);
        attr.register(1, key1.bind(&val1));

        let key2 = TestAlgebra::random_versor(2);
        let val2 = TestAlgebra::random_versor(2);
        attr.register(2, key2.bind(&val2));

        let attribution = attr.compute(&key1, &val1).unwrap();

        // Store 1 (matched) must outweigh store 2 (orthogonal).
        let c1 = attribution.contribution(1).unwrap_or(0.0);
        let c2 = attribution.contribution(2).unwrap_or(0.0);
        assert!(c1 > c2, "matched binding should dominate: c1={c1} c2={c2}");
    }

    #[test]
    fn attribution_filtering() {
        let result = AttributionResult {
            contributions: [(1, 0.5), (2, 0.3), (3, 0.1), (4, 0.05), (5, 0.05)]
                .into_iter()
                .collect(),
            top_contributors: vec![(1, 0.5), (2, 0.3), (3, 0.1), (4, 0.05), (5, 0.05)],
            total_mass: 1.0,
            is_approximate: false,
        };

        let above = result.above_threshold(0.2);
        assert_eq!(above.len(), 2);
        assert!(above.contains(&1));
        assert!(above.contains(&2));

        // top_n never overruns.
        assert_eq!(result.top_n(3).len(), 3);
        assert_eq!(result.top_n(10).len(), 5);
    }

    /// Verifies the additive `serde` feature-gating added during the WS 3
    /// port round-trips `AttributionResult` (the pre-v0.3.0 derive was
    /// unconditional; it is now `#[cfg_attr(feature = "serde", derive(...))]`).
    #[cfg(feature = "serde")]
    #[test]
    fn attribution_result_serde_roundtrip() {
        let original = AttributionResult {
            contributions: [(1, 0.6), (2, 0.4)].into_iter().collect(),
            top_contributors: vec![(1, 0.6), (2, 0.4)],
            total_mass: 1.0,
            is_approximate: false,
        };
        let encoded = bincode::serialize(&original).expect("serialize");
        let decoded: AttributionResult = bincode::deserialize(&encoded).expect("deserialize");
        assert_eq!(original.contributions, decoded.contributions);
        assert_eq!(original.top_contributors, decoded.top_contributors);
        assert_eq!(original.total_mass, decoded.total_mass);
        assert_eq!(original.is_approximate, decoded.is_approximate);
    }

    // ---- WS 4b: forward-mode dual (gradient) attribution ----

    /// Helper: register `n` random (key, value) bindings, returning the keys
    /// and values so tests can build the raw retrieved superposition.
    fn register_random(
        attr: &mut Attribution<TestAlgebra>,
        n: usize,
    ) -> (Vec<TestAlgebra>, Vec<TestAlgebra>) {
        let mut keys = Vec::new();
        let mut vals = Vec::new();
        for i in 0..n {
            let k = TestAlgebra::random_versor(2);
            let v = TestAlgebra::random_versor(2);
            attr.register(i as u64, k.bind(&v));
            keys.push(k);
            vals.push(v);
        }
        (keys, vals)
    }

    /// Core exactness guarantee: against the raw retrieved superposition
    /// `r = Σᵢ query⁻¹⊛bᵢ`, the gradient attributions sum to ~1.0.
    #[test]
    fn gradient_attribution_sums_to_one() {
        let mut attr = Attribution::<TestAlgebra>::new();
        let (keys, _vals) = register_random(&mut attr, 5);
        let query = keys[0].clone();

        // r = Σᵢ query⁻¹⊛bᵢ — the *pure-sum* superposition (component_add),
        // which is the idealization the exactness guarantee assumes. (A real
        // memory trace uses `bundle`, a softmax-weighted average, against
        // which the projections sum to < 1.)
        let mut r = TestAlgebra::zero();
        for (_id, binding) in &attr.bindings {
            let r_i = query.unbind(binding).unwrap();
            r = r.component_add(&r_i);
        }

        let result = attr.compute_gradient(&query, &r).unwrap();
        assert!(
            (result.total_mass - 1.0).abs() < 1e-6,
            "gradient attributions must sum to 1.0 against the pure-sum superposition, got {}",
            result.total_mass
        );
    }

    /// A single binding must attribute ~1.0 to itself (it *is* the whole result).
    #[test]
    fn gradient_single_binding_is_total() {
        let mut attr = Attribution::<TestAlgebra>::new();
        let key = TestAlgebra::random_versor(2);
        let val = TestAlgebra::random_versor(2);
        attr.register(0, key.bind(&val));

        let r = key.unbind(&attr.bindings[0].1).unwrap();
        let result = attr.compute_gradient(&key, &r).unwrap();
        assert!((result.contribution(0).unwrap() - 1.0).abs() < 1e-6);
    }

    /// Gradient attribution is energy-based: a store whose retrieved
    /// contribution has larger magnitude attributes more than a smaller one.
    /// (Contrast with `compute`, whose cosine heuristic is magnitude-blind.)
    #[test]
    fn gradient_attributes_larger_contribution_more() {
        let mut attr = Attribution::<TestAlgebra>::new();
        // Store 1: large value. Store 2: small value.
        let key1 = TestAlgebra::random_versor(2);
        let val1 = TestAlgebra::random_versor(2).component_scale(10.0);
        attr.register(1, key1.bind(&val1));
        let key2 = TestAlgebra::random_versor(2);
        let val2 = TestAlgebra::random_versor(2).component_scale(0.1);
        attr.register(2, key2.bind(&val2));

        // Use a neutral query (identity-like) so neither store is favored by
        // alignment; the magnitude difference should drive the attribution.
        let query = TestAlgebra::identity();
        let mut r = TestAlgebra::zero();
        for (_id, binding) in &attr.bindings {
            r = r.component_add(&query.unbind(binding).unwrap());
        }
        let result = attr.compute_gradient(&query, &r).unwrap();

        let c1 = result.contribution(1).unwrap_or(0.0);
        let c2 = result.contribution(2).unwrap_or(0.0);
        assert!(
            c1 > c2,
            "larger-magnitude store must attribute more (energy-based): c1={c1} c2={c2}"
        );
    }

    /// Gradient attribution is the orthogonal projection; it must match the
    /// formula `⟨rᵢ, r⟩/⟨r,r⟩` computed independently (no double-counting of
    /// shared mass, unlike the cosine heuristic in `compute`).
    #[test]
    fn gradient_matches_independent_projection() {
        let mut attr = Attribution::<TestAlgebra>::new();
        register_random(&mut attr, 4);
        let query = TestAlgebra::random_versor(2);

        let mut r = TestAlgebra::zero();
        let mut r_is = Vec::new();
        for (_id, binding) in &attr.bindings {
            let r_i = query.unbind(binding).unwrap();
            r_is.push(r_i.clone());
            r = r.bundle(&r_i, 1.0).unwrap();
        }
        let r_norm_sq = r.norm() * r.norm();

        let result = attr.compute_gradient(&query, &r).unwrap();
        for (i, r_i) in r_is.iter().enumerate() {
            let expected = inner_product(r_i, &r) / r_norm_sq;
            let got = result.contribution(i as u64).unwrap();
            assert!(
                (got - expected).abs() < 1e-9,
                "store {i}: gradient {got} must match projection {expected}"
            );
        }
    }

    /// A zero-norm result has no direction to project onto: no panic, empty
    /// contributions.
    #[test]
    fn gradient_zero_result_is_empty() {
        let mut attr = Attribution::<TestAlgebra>::new();
        register_random(&mut attr, 3);
        let query = TestAlgebra::random_versor(2);
        let zero = TestAlgebra::zero();
        let result = attr.compute_gradient(&query, &zero).unwrap();
        assert!(result.contributions.is_empty());
        assert_eq!(result.total_mass, 0.0);
    }
}
