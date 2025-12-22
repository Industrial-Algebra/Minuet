//! Parallel batch operations for holographic memory.
//!
//! Uses Rayon for data-parallel operations on TDC elements.

use amari_fusion::holographic::{Bindable, TropicalDualClifford};
use rayon::prelude::*;

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::precision::MinuetFloat;

/// Parallel batch binding.
///
/// Computes `keys[i] ⊛ values[i]` for all pairs in parallel.
///
/// # Panics
///
/// Panics if `keys.len() != values.len()`.
#[must_use]
pub fn bind_batch_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    keys: &[TropicalDualClifford<T, DIM>],
    values: &[TropicalDualClifford<T, DIM>],
) -> Vec<TropicalDualClifford<T, DIM>>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    assert_eq!(keys.len(), values.len(), "keys and values must have same length");

    keys.par_iter()
        .zip(values.par_iter())
        .map(|(k, v)| k.bind(v))
        .collect()
}

/// Parallel bundling with tree reduction.
///
/// Combines all items using bundling operation with the specified temperature.
///
/// # Arguments
///
/// * `items` - Items to bundle together
/// * `beta` - Temperature parameter for bundling
///
/// # Returns
///
/// The bundled result, or bundling zero if items is empty.
#[must_use]
pub fn bundle_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    items: &[TropicalDualClifford<T, DIM>],
    beta: T,
) -> TropicalDualClifford<T, DIM>
where
    TropicalDualClifford<T, DIM>: Send + Sync + Clone,
{
    if items.is_empty() {
        return TropicalDualClifford::bundling_zero();
    }

    items
        .par_iter()
        .cloned()
        .reduce(
            || TropicalDualClifford::bundling_zero(),
            |a, b| a.bundle(&b, beta),
        )
}

/// Parallel similarity computation.
///
/// Computes similarity between query and each candidate in parallel.
#[must_use]
pub fn similarities_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    query: &TropicalDualClifford<T, DIM>,
    candidates: &[TropicalDualClifford<T, DIM>],
) -> Vec<f64>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    candidates
        .par_iter()
        .map(|c| query.similarity(c).to_f64().unwrap_or(0.0))
        .collect()
}

/// Parallel unbinding.
///
/// Computes `keys[i].unbind(traces[i])` for all pairs in parallel.
#[must_use]
pub fn unbind_batch_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    keys: &[TropicalDualClifford<T, DIM>],
    traces: &[TropicalDualClifford<T, DIM>],
) -> Vec<TropicalDualClifford<T, DIM>>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    assert_eq!(keys.len(), traces.len(), "keys and traces must have same length");

    keys.par_iter()
        .zip(traces.par_iter())
        .map(|(k, t)| k.unbind(t))
        .collect()
}

/// Find top-k most similar candidates in parallel.
///
/// Returns (index, similarity) pairs sorted by descending similarity.
#[must_use]
pub fn top_k_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    query: &TropicalDualClifford<T, DIM>,
    candidates: &[TropicalDualClifford<T, DIM>],
    k: usize,
) -> Vec<(usize, f64)>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    let mut similarities: Vec<(usize, f64)> = candidates
        .par_iter()
        .enumerate()
        .map(|(idx, c)| (idx, query.similarity(c).to_f64().unwrap_or(0.0)))
        .collect();

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    similarities.truncate(k);

    similarities
}

/// Parallel normalization.
///
/// Normalizes all elements to unit magnitude in parallel.
#[must_use]
pub fn normalize_batch_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    items: &[TropicalDualClifford<T, DIM>],
) -> Vec<TropicalDualClifford<T, DIM>>
where
    TropicalDualClifford<T, DIM>: Send + Sync + Clone,
{
    items
        .par_iter()
        .map(|item| {
            let mag = item.magnitude();
            if mag > T::MIN_POSITIVE {
                item.scale(T::one() / mag)
            } else {
                item.clone()
            }
        })
        .collect()
}

/// Parallel grade projection.
///
/// Projects all elements to the specified grade in parallel.
#[must_use]
pub fn project_grade_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    items: &[TropicalDualClifford<T, DIM>],
    grade: usize,
) -> Vec<TropicalDualClifford<T, DIM>>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    items
        .par_iter()
        .map(|item| item.project_grade(grade))
        .collect()
}

/// Parallel weighted sum.
///
/// Computes `sum(items[i] * weights[i])` in parallel using tree reduction.
#[must_use]
pub fn weighted_sum_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    items: &[TropicalDualClifford<T, DIM>],
    weights: &[T],
) -> TropicalDualClifford<T, DIM>
where
    TropicalDualClifford<T, DIM>: Send + Sync + Clone,
{
    assert_eq!(items.len(), weights.len(), "items and weights must have same length");

    if items.is_empty() {
        return TropicalDualClifford::bundling_zero();
    }

    items
        .par_iter()
        .zip(weights.par_iter())
        .map(|(item, &weight)| item.scale(weight))
        .reduce(
            || TropicalDualClifford::bundling_zero(),
            |a, b| a.add(&b),
        )
}

/// Batch configuration for adaptive parallelism.
#[derive(Debug, Clone, Copy)]
pub struct BatchConfig {
    /// Minimum batch size for parallel execution.
    pub min_parallel_size: usize,

    /// Chunk size for parallel iteration.
    pub chunk_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            min_parallel_size: 100,
            chunk_size: 64,
        }
    }
}

impl BatchConfig {
    /// Check if parallel execution is beneficial for the given size.
    #[must_use]
    pub fn should_parallelize(&self, size: usize) -> bool {
        size >= self.min_parallel_size
    }
}

/// Adaptive batch binding that chooses sequential or parallel based on size.
#[must_use]
pub fn bind_batch_adaptive<T: MinuetFloat + Send + Sync, const DIM: usize>(
    keys: &[TropicalDualClifford<T, DIM>],
    values: &[TropicalDualClifford<T, DIM>],
    config: &BatchConfig,
) -> Vec<TropicalDualClifford<T, DIM>>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    if config.should_parallelize(keys.len()) {
        bind_batch_parallel(keys, values)
    } else {
        keys.iter()
            .zip(values.iter())
            .map(|(k, v)| k.bind(v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_binding() {
        let keys: Vec<TropicalDualClifford<f64, 64>> = (0..100)
            .map(|_| TropicalDualClifford::random())
            .collect();

        let values: Vec<TropicalDualClifford<f64, 64>> = (0..100)
            .map(|_| TropicalDualClifford::random())
            .collect();

        let results = bind_batch_parallel(&keys, &values);
        assert_eq!(results.len(), 100);

        // Verify correctness against sequential
        for i in 0..100 {
            let expected = keys[i].bind(&values[i]);
            assert!((results[i].similarity(&expected) - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn parallel_bundling() {
        let items: Vec<TropicalDualClifford<f64, 64>> = (0..50)
            .map(|_| TropicalDualClifford::random())
            .collect();

        let parallel_result = bundle_parallel(&items, 1.0);

        // Sequential bundling for comparison
        let mut sequential = TropicalDualClifford::bundling_zero();
        for item in &items {
            sequential = sequential.bundle(item, 1.0);
        }

        // Results should be similar (not exact due to different reduction order)
        let sim = parallel_result.similarity(&sequential);
        assert!(sim > 0.9);
    }

    #[test]
    fn parallel_similarities() {
        let query: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let candidates: Vec<TropicalDualClifford<f64, 64>> = (0..100)
            .map(|_| TropicalDualClifford::random())
            .collect();

        let sims = similarities_parallel(&query, &candidates);
        assert_eq!(sims.len(), 100);

        // All similarities should be finite
        for s in &sims {
            assert!(s.is_finite());
        }
    }

    #[test]
    fn top_k_finds_best() {
        let query: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();

        let mut candidates: Vec<TropicalDualClifford<f64, 64>> = (0..100)
            .map(|_| TropicalDualClifford::random())
            .collect();

        // Put query at position 50
        candidates[50] = query.clone();

        let top = top_k_parallel(&query, &candidates, 5);
        assert_eq!(top.len(), 5);

        // Position 50 should be first (highest similarity)
        assert_eq!(top[0].0, 50);
        assert!((top[0].1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn adaptive_batch() {
        let config = BatchConfig {
            min_parallel_size: 50,
            chunk_size: 64,
        };

        // Small batch - should use sequential
        let small_keys: Vec<TropicalDualClifford<f64, 64>> = (0..10)
            .map(|_| TropicalDualClifford::random())
            .collect();
        let small_values: Vec<TropicalDualClifford<f64, 64>> = (0..10)
            .map(|_| TropicalDualClifford::random())
            .collect();

        let small_results = bind_batch_adaptive(&small_keys, &small_values, &config);
        assert_eq!(small_results.len(), 10);

        // Large batch - should use parallel
        let large_keys: Vec<TropicalDualClifford<f64, 64>> = (0..100)
            .map(|_| TropicalDualClifford::random())
            .collect();
        let large_values: Vec<TropicalDualClifford<f64, 64>> = (0..100)
            .map(|_| TropicalDualClifford::random())
            .collect();

        let large_results = bind_batch_adaptive(&large_keys, &large_values, &config);
        assert_eq!(large_results.len(), 100);
    }
}
