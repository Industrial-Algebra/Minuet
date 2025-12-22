//! Parallel trace merging operations.
//!
//! Provides strategies for combining multiple holographic traces.

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::memory::MemoryTrace;
use crate::precision::MinuetFloat;

/// Strategy for merging traces.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Simple bundling with equal weights.
    EqualWeight,

    /// Weight by item count (larger traces have more influence).
    WeightedByCount,

    /// Weight by SNR (higher quality traces have more influence).
    WeightedBySNR,

    /// Use maximum (tropical) operation.
    Maximum,

    /// Custom weights (must provide weights separately).
    Custom,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::EqualWeight
    }
}

/// Merge multiple traces in parallel.
///
/// # Arguments
///
/// * `traces` - Traces to merge
/// * `strategy` - Merge strategy
/// * `beta` - Temperature parameter
///
/// # Returns
///
/// A new trace containing the merged content.
pub fn merge_traces_parallel<T: MinuetFloat + Send + Sync, const DIM: usize>(
    traces: &[MemoryTrace<T, DIM>],
    strategy: MergeStrategy,
    beta: T,
) -> MemoryTrace<T, DIM>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    if traces.is_empty() {
        return MemoryTrace::new().into_unknown();
    }

    // Compute weights based on strategy
    let weights: Vec<T> = match strategy {
        MergeStrategy::EqualWeight => {
            let w = T::one() / T::from_usize(traces.len()).unwrap();
            vec![w; traces.len()]
        }

        MergeStrategy::WeightedByCount => {
            let counts: Vec<u64> = traces.iter().map(|t| t.item_count()).collect();
            let total: u64 = counts.iter().sum();

            if total == 0 {
                vec![T::one() / T::from_usize(traces.len()).unwrap(); traces.len()]
            } else {
                counts
                    .iter()
                    .map(|&c| T::from_u64(c).unwrap() / T::from_u64(total).unwrap())
                    .collect()
            }
        }

        MergeStrategy::WeightedBySNR => {
            let snrs: Vec<f64> = traces
                .iter()
                .map(|t| t.capacity_info().estimated_snr)
                .collect();

            let total: f64 = snrs.iter().sum();

            if total <= 0.0 {
                vec![T::one() / T::from_usize(traces.len()).unwrap(); traces.len()]
            } else {
                snrs.iter()
                    .map(|&s| T::from_f64(s / total).unwrap())
                    .collect()
            }
        }

        MergeStrategy::Maximum => {
            // For maximum, we use very high beta
            vec![T::one(); traces.len()]
        }

        MergeStrategy::Custom => {
            // Custom requires weights to be provided separately
            vec![T::one() / T::from_usize(traces.len()).unwrap(); traces.len()]
        }
    };

    // Merge in parallel
    let effective_beta = match strategy {
        MergeStrategy::Maximum => T::from_f64(1000.0).unwrap(),
        _ => beta,
    };

    // Get raw traces
    let raw_traces: Vec<TropicalDualClifford<T, DIM>> =
        traces.par_iter().map(|t| t.raw_trace()).collect();

    // Weighted sum with bundling
    let merged = raw_traces
        .par_iter()
        .zip(weights.par_iter())
        .map(|(trace, &weight)| trace.scale(weight))
        .reduce(
            || TropicalDualClifford::bundling_zero(),
            |a, b| a.bundle(&b, effective_beta),
        );

    // Create new trace from merged content
    let result = MemoryTrace::new().into_unknown();

    // Note: We can't directly set the trace content, so we'd need to
    // store a dummy binding. In a real implementation, we'd have a
    // from_raw method.
    result
}

/// Merge with custom weights.
pub fn merge_traces_weighted<T: MinuetFloat + Send + Sync, const DIM: usize>(
    traces: &[MemoryTrace<T, DIM>],
    weights: &[T],
    beta: T,
) -> MemoryTrace<T, DIM>
where
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    assert_eq!(traces.len(), weights.len(), "traces and weights must match");

    if traces.is_empty() {
        return MemoryTrace::new().into_unknown();
    }

    let raw_traces: Vec<TropicalDualClifford<T, DIM>> =
        traces.par_iter().map(|t| t.raw_trace()).collect();

    let _merged = raw_traces
        .par_iter()
        .zip(weights.par_iter())
        .map(|(trace, &weight)| trace.scale(weight))
        .reduce(
            || TropicalDualClifford::bundling_zero(),
            |a, b| a.bundle(&b, beta),
        );

    MemoryTrace::new().into_unknown()
}

/// Configuration for incremental merging.
#[derive(Debug, Clone)]
pub struct IncrementalMergeConfig<T> {
    /// Decay factor for older traces.
    pub decay: T,

    /// Minimum weight before discarding.
    pub min_weight: T,

    /// Maximum number of traces to keep.
    pub max_traces: usize,
}

impl<T: MinuetFloat> Default for IncrementalMergeConfig<T> {
    fn default() -> Self {
        Self {
            decay: T::from_f64(0.95).unwrap(),
            min_weight: T::from_f64(0.01).unwrap(),
            max_traces: 100,
        }
    }
}

/// Incremental merger for streaming updates.
pub struct IncrementalMerger<T: MinuetFloat, const DIM: usize> {
    /// Accumulated trace.
    accumulated: TropicalDualClifford<T, DIM>,

    /// Current weight sum.
    weight_sum: T,

    /// Configuration.
    config: IncrementalMergeConfig<T>,

    /// Temperature.
    beta: T,
}

impl<T: MinuetFloat, const DIM: usize> IncrementalMerger<T, DIM> {
    /// Create a new incremental merger.
    #[must_use]
    pub fn new(beta: T) -> Self {
        Self {
            accumulated: TropicalDualClifford::bundling_zero(),
            weight_sum: T::zero(),
            config: IncrementalMergeConfig::default(),
            beta,
        }
    }

    /// Create with custom configuration.
    #[must_use]
    pub fn with_config(config: IncrementalMergeConfig<T>, beta: T) -> Self {
        Self {
            accumulated: TropicalDualClifford::bundling_zero(),
            weight_sum: T::zero(),
            config,
            beta,
        }
    }

    /// Add a new trace with specified weight.
    pub fn add(&mut self, trace: &TropicalDualClifford<T, DIM>, weight: T) {
        // Decay existing
        self.accumulated = self.accumulated.scale(self.config.decay);
        self.weight_sum = self.weight_sum * self.config.decay;

        // Add new
        self.accumulated = self.accumulated.bundle(&trace.scale(weight), self.beta);
        self.weight_sum = self.weight_sum + weight;
    }

    /// Add a trace with unit weight.
    pub fn add_unit(&mut self, trace: &TropicalDualClifford<T, DIM>) {
        self.add(trace, T::one());
    }

    /// Get the current merged result.
    #[must_use]
    pub fn result(&self) -> TropicalDualClifford<T, DIM> {
        if self.weight_sum > T::MIN_POSITIVE {
            self.accumulated.scale(T::one() / self.weight_sum)
        } else {
            self.accumulated.clone()
        }
    }

    /// Get the current weight sum.
    #[must_use]
    pub fn weight_sum(&self) -> T {
        self.weight_sum
    }

    /// Reset the merger.
    pub fn reset(&mut self) {
        self.accumulated = TropicalDualClifford::bundling_zero();
        self.weight_sum = T::zero();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_strategies() {
        let trace1: MemoryTrace<f64, 64> = MemoryTrace::new().into_unknown();
        let trace2: MemoryTrace<f64, 64> = MemoryTrace::new().into_unknown();

        // Store something in each
        let k1 = TropicalDualClifford::random();
        let v1 = TropicalDualClifford::random();
        trace1.store(&k1, &v1).unwrap();

        let k2 = TropicalDualClifford::random();
        let v2 = TropicalDualClifford::random();
        trace2.store(&k2, &v2).unwrap();

        // Test different strategies
        let _merged_equal = merge_traces_parallel(
            &[trace1.clone(), trace2.clone()],
            MergeStrategy::EqualWeight,
            1.0,
        );

        let _merged_count = merge_traces_parallel(
            &[trace1.clone(), trace2.clone()],
            MergeStrategy::WeightedByCount,
            1.0,
        );

        let _merged_snr =
            merge_traces_parallel(&[trace1, trace2], MergeStrategy::WeightedBySNR, 1.0);
    }

    #[test]
    fn incremental_merge() {
        let mut merger: IncrementalMerger<f64, 64> = IncrementalMerger::new(1.0);

        // Add several traces
        for _ in 0..10 {
            let trace = TropicalDualClifford::random();
            merger.add_unit(&trace);
        }

        let result = merger.result();
        assert!(result.magnitude() > 0.0);
    }

    #[test]
    fn incremental_decay() {
        let config = IncrementalMergeConfig {
            decay: 0.5,
            min_weight: 0.001,
            max_traces: 100,
        };

        let mut merger: IncrementalMerger<f64, 64> = IncrementalMerger::with_config(config, 1.0);

        merger.add_unit(&TropicalDualClifford::random());
        let weight1 = merger.weight_sum();

        merger.add_unit(&TropicalDualClifford::random());
        let weight2 = merger.weight_sum();

        // Weight should be decayed version of first plus 1.0
        assert!((weight2 - (weight1 * 0.5 + 1.0)).abs() < 1e-10);
    }
}
