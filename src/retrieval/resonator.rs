//! Resonator cleanup networks for holographic retrieval.
//!
//! Resonators iteratively project noisy retrievals onto the codebook manifold,
//! cleaning up interference from other stored items.

use std::marker::PhantomData;

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::binding::Codebook;
use crate::error::{MinuetError, Result};
use crate::precision::MinuetFloat;

use super::temperature::Temperature;

/// Configuration for resonator cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonatorConfig {
    /// Maximum iterations before giving up.
    pub max_iterations: usize,

    /// Similarity threshold for convergence.
    pub convergence_threshold: f64,

    /// Temperature schedule for iterations.
    pub temperature: Temperature,

    /// Whether to normalize after each iteration.
    pub normalize: bool,

    /// Early stopping: stop if similarity doesn't improve by this much.
    pub min_improvement: f64,
}

impl Default for ResonatorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            convergence_threshold: 0.99,
            temperature: Temperature::soft(),
            normalize: true,
            min_improvement: 1e-6,
        }
    }
}

impl ResonatorConfig {
    /// Create with specific max iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set convergence threshold.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.convergence_threshold = threshold;
        self
    }

    /// Set temperature.
    #[must_use]
    pub fn with_temperature(mut self, temp: Temperature) -> Self {
        self.temperature = temp;
        self
    }

    /// Enable/disable normalization.
    #[must_use]
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }
}

/// Result of resonator cleanup.
#[derive(Debug, Clone)]
pub struct ResonatorResult<T: MinuetFloat, const DIM: usize> {
    /// The cleaned-up value.
    pub value: TropicalDualClifford<T, DIM>,

    /// Whether convergence was achieved.
    pub converged: bool,

    /// Number of iterations performed.
    pub iterations: usize,

    /// Final similarity to best codebook match.
    pub final_similarity: f64,

    /// Index of best match in codebook (if converged).
    pub best_match_index: Option<usize>,

    /// Similarity history over iterations.
    pub similarity_history: Vec<f64>,
}

/// A resonator cleanup network.
///
/// Resonators iteratively project noisy inputs onto a codebook manifold,
/// using the binding algebra to clean up retrieval noise.
pub struct Resonator<T: MinuetFloat, const DIM: usize> {
    /// Reference codebook for cleanup targets.
    codebook_symbols: Vec<TropicalDualClifford<T, DIM>>,

    /// Configuration.
    config: ResonatorConfig,

    _phantom: PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> Resonator<T, DIM> {
    /// Create a new resonator with the given codebook.
    #[must_use]
    pub fn new(codebook: &Codebook<T, DIM>) -> Self {
        Self {
            codebook_symbols: codebook.all_symbols(),
            config: ResonatorConfig::default(),
        }
    }

    /// Create with specific configuration.
    #[must_use]
    pub fn with_config(codebook: &Codebook<T, DIM>, config: ResonatorConfig) -> Self {
        Self {
            codebook_symbols: codebook.all_symbols(),
            config,
            _phantom: PhantomData,
        }
    }

    /// Create from raw symbol vectors.
    #[must_use]
    pub fn from_symbols(
        symbols: Vec<TropicalDualClifford<T, DIM>>,
        config: ResonatorConfig,
    ) -> Self {
        Self {
            codebook_symbols: symbols,
            config,
            _phantom: PhantomData,
        }
    }

    /// Clean up a noisy input.
    ///
    /// Iteratively projects the input onto the codebook manifold
    /// using similarity-weighted averaging.
    pub fn cleanup(&self, input: &TropicalDualClifford<T, DIM>) -> Result<ResonatorResult<T, DIM>> {
        if self.codebook_symbols.is_empty() {
            return Err(MinuetError::CodebookInvariant(
                "Resonator codebook is empty".into(),
            ));
        }

        let mut current = input.clone();
        let mut similarity_history = Vec::with_capacity(self.config.max_iterations);
        let mut best_similarity = 0.0f64;
        let mut best_match_index = 0;
        let mut prev_similarity = 0.0f64;

        for iteration in 0..self.config.max_iterations {
            // Compute similarities to all codebook symbols
            let similarities: Vec<f64> = self
                .codebook_symbols
                .iter()
                .map(|s| current.similarity(s).to_f64().unwrap_or(0.0))
                .collect();

            // Find best match
            let (idx, &max_sim) = similarities
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            similarity_history.push(max_sim);

            if max_sim > best_similarity {
                best_similarity = max_sim;
                best_match_index = idx;
            }

            // Check convergence
            if max_sim >= self.config.convergence_threshold {
                return Ok(ResonatorResult {
                    value: self.codebook_symbols[idx].clone(),
                    converged: true,
                    iterations: iteration + 1,
                    final_similarity: max_sim,
                    best_match_index: Some(idx),
                    similarity_history,
                });
            }

            // Check for stagnation
            if iteration > 0 && (max_sim - prev_similarity).abs() < self.config.min_improvement {
                // Not improving, return best so far
                return Ok(ResonatorResult {
                    value: self.codebook_symbols[best_match_index].clone(),
                    converged: false,
                    iterations: iteration + 1,
                    final_similarity: best_similarity,
                    best_match_index: Some(best_match_index),
                    similarity_history,
                });
            }

            prev_similarity = max_sim;

            // Compute weighted average (resonator update)
            let beta = self.config.temperature.beta_at(iteration);
            current = self.weighted_average(&similarities, beta);

            // Optionally normalize
            if self.config.normalize {
                let mag = current.magnitude();
                if mag > T::MIN_POSITIVE {
                    current = current.scale(T::one() / mag);
                }
            }
        }

        // Did not converge within max iterations
        Err(MinuetError::ResonatorDidNotConverge {
            iterations: self.config.max_iterations,
            final_similarity: best_similarity,
        })
    }

    /// Compute weighted average of codebook symbols.
    fn weighted_average(&self, similarities: &[f64], beta: f64) -> TropicalDualClifford<T, DIM> {
        // Apply softmax with temperature
        let max_sim = similarities
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let exp_sims: Vec<f64> = similarities
            .iter()
            .map(|s| ((s - max_sim) * beta).exp())
            .collect();

        let sum_exp: f64 = exp_sims.iter().sum();

        // Weighted sum
        let mut result = TropicalDualClifford::bundling_zero();

        for (symbol, &weight) in self.codebook_symbols.iter().zip(exp_sims.iter()) {
            let normalized_weight = T::from_f64(weight / sum_exp).unwrap();
            result = result.add(&symbol.scale(normalized_weight));
        }

        result
    }

    /// Clean up multiple inputs in batch.
    pub fn cleanup_batch(
        &self,
        inputs: &[TropicalDualClifford<T, DIM>],
    ) -> Vec<Result<ResonatorResult<T, DIM>>> {
        inputs.iter().map(|input| self.cleanup(input)).collect()
    }

    /// Find the nearest codebook symbol without iteration.
    #[must_use]
    pub fn nearest(&self, input: &TropicalDualClifford<T, DIM>) -> Option<(usize, f64)> {
        if self.codebook_symbols.is_empty() {
            return None;
        }

        self.codebook_symbols
            .iter()
            .enumerate()
            .map(|(idx, s)| (idx, input.similarity(s).to_f64().unwrap_or(0.0)))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    }

    /// Get the codebook size.
    #[must_use]
    pub fn codebook_size(&self) -> usize {
        self.codebook_symbols.len()
    }

    /// Update the codebook symbols.
    pub fn set_symbols(&mut self, symbols: Vec<TropicalDualClifford<T, DIM>>) {
        self.codebook_symbols = symbols;
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &ResonatorConfig {
        &self.config
    }
}

/// Multi-stage resonator for hierarchical cleanup.
///
/// Applies multiple resonators in sequence, from coarse to fine codebooks.
pub struct HierarchicalResonator<T: MinuetFloat, const DIM: usize> {
    stages: Vec<Resonator<T, DIM>>,
}

impl<T: MinuetFloat, const DIM: usize> HierarchicalResonator<T, DIM> {
    /// Create from a sequence of resonators.
    #[must_use]
    pub fn new(stages: Vec<Resonator<T, DIM>>) -> Self {
        Self { stages }
    }

    /// Clean up through all stages.
    pub fn cleanup(&self, input: &TropicalDualClifford<T, DIM>) -> Result<ResonatorResult<T, DIM>> {
        let mut current = input.clone();
        let mut total_iterations = 0;
        let mut all_history = Vec::new();

        for stage in &self.stages {
            let result = stage.cleanup(&current)?;
            total_iterations += result.iterations;
            all_history.extend(result.similarity_history);
            current = result.value;
        }

        // Final nearest check
        let last_stage = self.stages.last().unwrap();
        let (idx, sim) = last_stage.nearest(&current).unwrap();

        Ok(ResonatorResult {
            value: current,
            converged: sim >= last_stage.config.convergence_threshold,
            iterations: total_iterations,
            final_similarity: sim,
            best_match_index: Some(idx),
            similarity_history: all_history,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonator_cleanup() {
        let codebook: Codebook<f64, 64> = Codebook::new();

        // Create some symbols
        let _a = codebook.symbol("a");
        let _b = codebook.symbol("b");
        let _c = codebook.symbol("c");

        let resonator = Resonator::new(&codebook);

        // Query with one of the symbols (should converge immediately)
        let query = codebook.get("a").unwrap();
        let result = resonator.cleanup(&query).unwrap();

        assert!(result.converged);
        assert!(result.final_similarity > 0.99);
    }

    #[test]
    fn resonator_with_noise() {
        let codebook: Codebook<f64, 64> = Codebook::new();

        let a = codebook.symbol("a");
        let b = codebook.symbol("b");

        // Create noisy version of 'a'
        let noise: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
        let noisy_a = a.scale(0.9).add(&noise.scale(0.1));

        let resonator = Resonator::with_config(
            &codebook,
            ResonatorConfig::default()
                .with_max_iterations(50)
                .with_threshold(0.9),
        );

        let result = resonator.cleanup(&noisy_a).unwrap();

        // Should clean up to 'a'
        let cleaned_sim = result.value.similarity(&a);
        assert!(cleaned_sim > 0.8);
    }

    #[test]
    fn nearest_without_iteration() {
        let codebook: Codebook<f64, 64> = Codebook::new();

        let _a = codebook.symbol("a");
        let _b = codebook.symbol("b");

        let resonator = Resonator::new(&codebook);

        let query = codebook.get("a").unwrap();
        let (idx, sim) = resonator.nearest(&query).unwrap();

        assert!(sim > 0.99);
    }

    #[test]
    fn config_builder() {
        let config = ResonatorConfig::default()
            .with_max_iterations(200)
            .with_threshold(0.95)
            .with_normalize(false);

        assert_eq!(config.max_iterations, 200);
        assert!((config.convergence_threshold - 0.95).abs() < 1e-10);
        assert!(!config.normalize);
    }
}
