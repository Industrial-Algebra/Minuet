//! Query builder and execution for holographic memory.
//!
//! Queries are structured operations over holographic memory, including
//! direct lookups, analogies, transformations, and composite patterns.

use std::time::{Duration, Instant};

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::{MinuetError, Result};
use crate::precision::MinuetFloat;
use crate::retrieval::Temperature;

use super::trace::MemoryTrace;

/// A bitmask for partial queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitMask {
    /// Indices of known components.
    pub known: Vec<usize>,
    /// Indices of unknown (masked) components.
    pub unknown: Vec<usize>,
}

impl BitMask {
    /// Create a mask where only specified indices are known.
    #[must_use]
    pub fn known_indices(known: Vec<usize>, total_dim: usize) -> Self {
        let unknown: Vec<usize> = (0..total_dim).filter(|i| !known.contains(i)).collect();
        Self { known, unknown }
    }

    /// Create a mask where only specified indices are unknown.
    #[must_use]
    pub fn unknown_indices(unknown: Vec<usize>, total_dim: usize) -> Self {
        let known: Vec<usize> = (0..total_dim).filter(|i| !unknown.contains(i)).collect();
        Self { known, unknown }
    }

    /// Check if an index is known.
    #[must_use]
    pub fn is_known(&self, idx: usize) -> bool {
        self.known.contains(&idx)
    }
}

/// Query pattern types.
#[derive(Debug, Clone)]
pub enum QueryPattern<T: MinuetFloat, const DIM: usize> {
    /// Direct key lookup.
    Key(TropicalDualClifford<T, DIM>),

    /// Analogy: find X such that X:target_context :: source:source_context.
    ///
    /// Computes: target_context ⊛ source_context⁻¹ ⊛ source
    Analogy {
        /// The source element.
        source: TropicalDualClifford<T, DIM>,
        /// The context of the source.
        source_context: TropicalDualClifford<T, DIM>,
        /// The target context to find the analogue in.
        target_context: TropicalDualClifford<T, DIM>,
    },

    /// Transformation: find items that result from applying transform to source.
    Transform {
        /// The source element.
        source: TropicalDualClifford<T, DIM>,
        /// The transformation to apply.
        transform: TropicalDualClifford<T, DIM>,
    },

    /// Partial: query with incomplete/masked key.
    Partial {
        /// The partial key (with some components filled).
        partial_key: TropicalDualClifford<T, DIM>,
        /// Mask indicating known vs unknown components.
        mask: BitMask,
    },

    /// Composite: combine multiple query patterns with weights.
    Composite(Vec<(QueryPattern<T, DIM>, f64)>),
}

/// Cleanup strategy for query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupStrategy {
    /// No cleanup, return raw retrieval.
    None,
    /// Project to nearest item in codebook.
    Resonator {
        /// Maximum iterations.
        max_iterations: usize,
        /// Convergence threshold.
        threshold: f64,
    },
    /// Single-step projection to versor manifold.
    VersorProjection,
}

impl Default for CleanupStrategy {
    fn default() -> Self {
        Self::None
    }
}

/// A structured query against holographic memory.
#[derive(Debug, Clone)]
pub struct Query<T: MinuetFloat, const DIM: usize> {
    /// The query pattern.
    pub(crate) pattern: QueryPattern<T, DIM>,
    /// Temperature for retrieval.
    pub(crate) temperature: Temperature,
    /// Cleanup strategy.
    pub(crate) cleanup: CleanupStrategy,
    /// Limit on number of results.
    pub(crate) limit: Option<usize>,
    /// Similarity threshold for results.
    pub(crate) threshold: Option<f64>,
}

impl<T: MinuetFloat, const DIM: usize> Query<T, DIM> {
    /// Create a direct key lookup query.
    #[must_use]
    pub fn key(key: TropicalDualClifford<T, DIM>) -> Self {
        Self {
            pattern: QueryPattern::Key(key),
            temperature: Temperature::default(),
            cleanup: CleanupStrategy::default(),
            limit: None,
            threshold: None,
        }
    }

    /// Create an analogy query: "X is to target_context as source is to source_context".
    ///
    /// This finds the value stored with a key that is analogically related
    /// to the source via the transformation implied by the context relationship.
    #[must_use]
    pub fn analogy(
        source: TropicalDualClifford<T, DIM>,
        source_context: TropicalDualClifford<T, DIM>,
        target_context: TropicalDualClifford<T, DIM>,
    ) -> Self {
        Self {
            pattern: QueryPattern::Analogy {
                source,
                source_context,
                target_context,
            },
            temperature: Temperature::default(),
            cleanup: CleanupStrategy::default(),
            limit: None,
            threshold: None,
        }
    }

    /// Create a transformation query.
    ///
    /// Finds values stored with keys that are transformations of the source.
    #[must_use]
    pub fn transform(
        source: TropicalDualClifford<T, DIM>,
        transform: TropicalDualClifford<T, DIM>,
    ) -> Self {
        Self {
            pattern: QueryPattern::Transform { source, transform },
            temperature: Temperature::default(),
            cleanup: CleanupStrategy::default(),
            limit: None,
            threshold: None,
        }
    }

    /// Create a partial query with known and unknown components.
    #[must_use]
    pub fn partial(partial_key: TropicalDualClifford<T, DIM>, mask: BitMask) -> Self {
        Self {
            pattern: QueryPattern::Partial { partial_key, mask },
            temperature: Temperature::default(),
            cleanup: CleanupStrategy::default(),
            limit: None,
            threshold: None,
        }
    }

    /// Create a composite query from multiple weighted patterns.
    #[must_use]
    pub fn composite(patterns: Vec<(QueryPattern<T, DIM>, f64)>) -> Self {
        Self {
            pattern: QueryPattern::Composite(patterns),
            temperature: Temperature::default(),
            cleanup: CleanupStrategy::default(),
            limit: None,
            threshold: None,
        }
    }

    /// Set retrieval temperature.
    #[must_use]
    pub fn with_temperature(mut self, temp: Temperature) -> Self {
        self.temperature = temp;
        self
    }

    /// Set cleanup strategy.
    #[must_use]
    pub fn with_cleanup(mut self, strategy: CleanupStrategy) -> Self {
        self.cleanup = strategy;
        self
    }

    /// Limit number of results.
    #[must_use]
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set similarity threshold.
    #[must_use]
    pub fn threshold(mut self, t: f64) -> Self {
        self.threshold = Some(t);
        self
    }

    /// Execute the query against a memory trace.
    pub(crate) fn execute<S>(self, trace: &MemoryTrace<T, DIM, S>) -> Result<QueryResult<T, DIM>> {
        let start = Instant::now();

        // Compute the query key based on pattern
        let query_key = self.compute_query_key()?;

        // Retrieve from trace
        let raw_result = trace.retrieve(&query_key);

        // Apply cleanup if specified
        let cleaned_result = match &self.cleanup {
            CleanupStrategy::None => raw_result,
            CleanupStrategy::Resonator {
                max_iterations,
                threshold,
            } => {
                // Resonator cleanup would go here
                // For now, return raw
                raw_result
            }
            CleanupStrategy::VersorProjection => {
                // Versor projection would go here
                raw_result
            }
        };

        let elapsed = start.elapsed();

        // Compute similarity/confidence
        let confidence = trace.capacity_info().estimated_snr;

        let result = RankedResult {
            value: cleaned_result,
            similarity: confidence,
            attribution: Vec::new(), // Attribution computed on demand
        };

        Ok(QueryResult {
            results: vec![result],
            stats: QueryStats {
                query_time: elapsed,
                items_scanned: trace.item_count() as usize,
                cleanup_iterations: match &self.cleanup {
                    CleanupStrategy::Resonator { max_iterations, .. } => Some(*max_iterations),
                    _ => None,
                },
            },
        })
    }

    /// Compute the effective query key from the pattern.
    fn compute_query_key(&self) -> Result<TropicalDualClifford<T, DIM>> {
        match &self.pattern {
            QueryPattern::Key(key) => Ok(key.clone()),

            QueryPattern::Analogy {
                source,
                source_context,
                target_context,
            } => {
                // Analogy: target_context ⊛ source_context⁻¹ ⊛ source
                // This extracts the transformation from source to source_context
                // and applies it in the target_context
                let transform = target_context.bind(&source_context.binding_inverse());
                Ok(transform.bind(source))
            }

            QueryPattern::Transform { source, transform } => {
                // Apply transformation
                Ok(transform.bind(source))
            }

            QueryPattern::Partial { partial_key, mask } => {
                // For partial queries, we use the partial key directly
                // The resonator cleanup will handle filling in unknowns
                Ok(partial_key.clone())
            }

            QueryPattern::Composite(patterns) => {
                if patterns.is_empty() {
                    return Err(MinuetError::InvalidQuery(
                        "Composite query must have at least one pattern".into(),
                    ));
                }

                // Compute weighted combination of pattern keys
                let mut combined = TropicalDualClifford::bundling_zero();
                for (pattern, weight) in patterns {
                    let sub_query = Query {
                        pattern: pattern.clone(),
                        temperature: self.temperature.clone(),
                        cleanup: CleanupStrategy::None,
                        limit: None,
                        threshold: None,
                    };
                    let key = sub_query.compute_query_key()?;
                    let beta = T::from_f64(*weight).unwrap();
                    combined = combined.bundle(&key, beta);
                }
                Ok(combined)
            }
        }
    }
}

/// Result of a query operation.
#[derive(Clone, Debug)]
pub struct QueryResult<T: MinuetFloat, const DIM: usize> {
    /// Retrieved values, ranked by relevance.
    pub results: Vec<RankedResult<T, DIM>>,
    /// Query execution statistics.
    pub stats: QueryStats,
}

impl<T: MinuetFloat, const DIM: usize> QueryResult<T, DIM> {
    /// Get the top result if any.
    #[must_use]
    pub fn top(&self) -> Option<&RankedResult<T, DIM>> {
        self.results.first()
    }

    /// Check if the query returned any results.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Number of results.
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }
}

/// A single ranked result from a query.
#[derive(Clone, Debug)]
pub struct RankedResult<T: MinuetFloat, const DIM: usize> {
    /// The retrieved value.
    pub value: TropicalDualClifford<T, DIM>,
    /// Similarity score.
    pub similarity: f64,
    /// Attribution: (store_id, contribution) pairs.
    pub attribution: Vec<(u64, f64)>,
}

/// Statistics about query execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryStats {
    /// Total query execution time.
    pub query_time: Duration,
    /// Number of items scanned.
    pub items_scanned: usize,
    /// Number of cleanup iterations (if applicable).
    pub cleanup_iterations: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_query_construction() {
        let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let query = Query::key(key.clone()).limit(5).threshold(0.8);

        assert!(matches!(query.pattern, QueryPattern::Key(_)));
        assert_eq!(query.limit, Some(5));
        assert_eq!(query.threshold, Some(0.8));
    }

    #[test]
    fn analogy_query_construction() {
        let source: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let source_ctx: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let target_ctx: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let query = Query::analogy(source, source_ctx, target_ctx).with_cleanup(
            CleanupStrategy::Resonator {
                max_iterations: 10,
                threshold: 0.95,
            },
        );

        assert!(matches!(query.pattern, QueryPattern::Analogy { .. }));
    }

    #[test]
    fn bitmask_construction() {
        let mask = BitMask::known_indices(vec![0, 1, 2], 10);
        assert!(mask.is_known(0));
        assert!(mask.is_known(1));
        assert!(mask.is_known(2));
        assert!(!mask.is_known(3));
        assert_eq!(mask.unknown.len(), 7);
    }

    #[test]
    fn composite_query() {
        let key1: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let key2: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let patterns = vec![
            (QueryPattern::Key(key1), 0.7),
            (QueryPattern::Key(key2), 0.3),
        ];

        let query = Query::composite(patterns);
        assert!(matches!(query.pattern, QueryPattern::Composite(_)));
    }
}
