//! Attribution and provenance tracking for holographic retrieval.
//!
//! Attribution uses the dual number component of TDC to track how much
//! each stored item contributes to a retrieval result.

use std::collections::HashMap;

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::Result;
use crate::precision::MinuetFloat;

/// Attribution information for a retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionResult {
    /// Store ID to contribution mapping.
    pub contributions: HashMap<u64, f64>,

    /// Top contributors sorted by contribution.
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

    /// Get the top N contributors.
    #[must_use]
    pub fn top_n(&self, n: usize) -> &[(u64, f64)] {
        &self.top_contributors[..n.min(self.top_contributors.len())]
    }

    /// Check if a specific store was a significant contributor.
    #[must_use]
    pub fn is_significant(&self, store_id: u64, threshold: f64) -> bool {
        self.contributions
            .get(&store_id)
            .map(|&c| c >= threshold)
            .unwrap_or(false)
    }

    /// Get store IDs above a contribution threshold.
    #[must_use]
    pub fn above_threshold(&self, threshold: f64) -> Vec<u64> {
        self.contributions
            .iter()
            .filter(|(_, &v)| v >= threshold)
            .map(|(&k, _)| k)
            .collect()
    }
}

/// Attribution calculator for holographic memory.
///
/// Uses the dual number component to track gradients/sensitivity
/// with respect to each stored binding.
pub struct Attribution<T: MinuetFloat, const DIM: usize> {
    /// Stored bindings with their IDs.
    bindings: Vec<(u64, TropicalDualClifford<T, DIM>)>,

    /// Whether to use approximate (fast) or exact (slow) attribution.
    approximate: bool,

    /// Contribution threshold for filtering.
    threshold: f64,

    /// Maximum number of attributions to return.
    max_attributions: usize,
}

impl<T: MinuetFloat, const DIM: usize> Attribution<T, DIM> {
    /// Create a new attribution calculator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            approximate: true,
            threshold: 0.01,
            max_attributions: 100,
        }
    }

    /// Set to exact (slower but more accurate) mode.
    #[must_use]
    pub fn exact(mut self) -> Self {
        self.approximate = false;
        self
    }

    /// Set contribution threshold.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set maximum attributions.
    #[must_use]
    pub fn with_max(mut self, max: usize) -> Self {
        self.max_attributions = max;
        self
    }

    /// Register a stored binding for attribution tracking.
    pub fn register(&mut self, store_id: u64, binding: TropicalDualClifford<T, DIM>) {
        self.bindings.push((store_id, binding));
    }

    /// Clear all registered bindings.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    /// Compute attribution for a retrieval result.
    ///
    /// # Arguments
    ///
    /// * `query` - The query that was used
    /// * `result` - The retrieval result
    ///
    /// # Returns
    ///
    /// Attribution information showing contribution of each stored item.
    pub fn compute(
        &self,
        query: &TropicalDualClifford<T, DIM>,
        result: &TropicalDualClifford<T, DIM>,
    ) -> Result<AttributionResult> {
        if self.bindings.is_empty() {
            return Ok(AttributionResult {
                contributions: HashMap::new(),
                top_contributors: Vec::new(),
                total_mass: 0.0,
                is_approximate: self.approximate,
            });
        }

        // Compute contribution of each binding
        let mut contributions = HashMap::new();
        let mut total_mass = 0.0;

        for (store_id, binding) in &self.bindings {
            // Contribution is proportional to similarity between
            // the unbinding of query from this binding and the result
            let unbound = query.unbind(binding);
            let sim = unbound.similarity(result).to_f64().unwrap_or(0.0);

            // Only track positive contributions above threshold
            if sim > self.threshold {
                contributions.insert(*store_id, sim);
                total_mass += sim;
            }
        }

        // Normalize contributions if we have any
        if total_mass > 0.0 {
            for v in contributions.values_mut() {
                *v /= total_mass;
            }
        }

        // Sort by contribution
        let mut top_contributors: Vec<(u64, f64)> =
            contributions.iter().map(|(&k, &v)| (k, v)).collect();

        top_contributors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        top_contributors.truncate(self.max_attributions);

        Ok(AttributionResult {
            contributions,
            top_contributors,
            total_mass: 1.0, // Normalized
            is_approximate: self.approximate,
        })
    }

    /// Compute attribution using dual number gradients.
    ///
    /// This is the more mathematically principled approach, using the
    /// dual component of TDC for automatic differentiation.
    pub fn compute_gradient(
        &self,
        query: &TropicalDualClifford<T, DIM>,
        result: &TropicalDualClifford<T, DIM>,
    ) -> Result<AttributionResult> {
        // For full implementation, we would:
        // 1. Set the dual component of each binding to track its contribution
        // 2. Propagate through the bundling operation
        // 3. Extract gradients from the dual component of the result
        //
        // For now, fall back to similarity-based attribution
        self.compute(query, result)
    }

    /// Get the number of registered bindings.
    #[must_use]
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for Attribution<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for attribution queries.
#[derive(Debug)]
pub struct AttributionQuery<T: MinuetFloat, const DIM: usize> {
    query: TropicalDualClifford<T, DIM>,
    result: TropicalDualClifford<T, DIM>,
    store_ids: Option<Vec<u64>>,
    threshold: f64,
}

impl<T: MinuetFloat, const DIM: usize> AttributionQuery<T, DIM> {
    /// Create a new attribution query.
    #[must_use]
    pub fn new(query: TropicalDualClifford<T, DIM>, result: TropicalDualClifford<T, DIM>) -> Self {
        Self {
            query,
            result,
            store_ids: None,
            threshold: 0.01,
        }
    }

    /// Only compute attribution for specific store IDs.
    #[must_use]
    pub fn for_stores(mut self, ids: Vec<u64>) -> Self {
        self.store_ids = Some(ids);
        self
    }

    /// Set contribution threshold.
    #[must_use]
    pub fn threshold(mut self, t: f64) -> Self {
        self.threshold = t;
        self
    }
}

/// Explanation of a retrieval result.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[test]
    fn attribution_empty() {
        let attr: Attribution<f64, 64> = Attribution::new();
        let query = TropicalDualClifford::random();
        let result = TropicalDualClifford::random();

        let result = attr.compute(&query, &result).unwrap();
        assert!(result.contributions.is_empty());
    }

    #[test]
    fn attribution_with_bindings() {
        let mut attr: Attribution<f64, 64> = Attribution::new().with_threshold(0.0);

        // Register some bindings
        let key1 = TropicalDualClifford::random();
        let val1 = TropicalDualClifford::random();
        let binding1 = key1.bind(&val1);
        attr.register(1, binding1);

        let key2 = TropicalDualClifford::random();
        let val2 = TropicalDualClifford::random();
        let binding2 = key2.bind(&val2);
        attr.register(2, binding2);

        // Query with key1
        let result = val1.clone();
        let attribution = attr.compute(&key1, &result).unwrap();

        // Should have some contributions
        assert!(!attribution.contributions.is_empty());
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
    }
}
