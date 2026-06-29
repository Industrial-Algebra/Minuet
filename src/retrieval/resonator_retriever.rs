// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0
//! Resonator-based retriever - uses cleanup network.

use amari_holographic::{BindingAlgebra, Resonator, ResonatorConfig};

use crate::error::{MinuetError, MinuetResult};
use crate::traits::{CleanupResult, RetrievalContext, Retriever};

/// A retriever that uses a resonator network for cleanup.
///
/// This retriever uses amari-holographic's Resonator to clean up
/// noisy retrieval results by iteratively projecting onto a codebook.
///
/// # Example
///
/// ```rust
/// # use minuet::prelude::*;
/// # use minuet::retrieval::ResonatorRetriever;
/// # use minuet::encoding::HashMapCodebook;
/// # type Algebra = ProductCliffordAlgebra<64>;
/// # fn main() -> MinuetResult<()> {
/// let codebook = HashMapCodebook::<Algebra>::new();
/// // ... populate codebook ...
/// # let _sym = codebook.symbol("test");
///
/// let retriever = ResonatorRetriever::from_symbols(codebook.all_symbols())?;
/// # Ok(())
/// # }
/// ```
pub struct ResonatorRetriever<A: BindingAlgebra> {
    resonator: Option<Resonator<A>>,
    config: ResonatorConfig,
}

impl<A: BindingAlgebra> Default for ResonatorRetriever<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> ResonatorRetriever<A> {
    /// Create a new resonator retriever without a codebook.
    ///
    /// The codebook must be provided via the context at cleanup time.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resonator: None,
            config: ResonatorConfig::default(),
        }
    }

    /// Create with a pre-built resonator.
    #[must_use]
    pub fn with_resonator(resonator: Resonator<A>) -> Self {
        Self {
            resonator: Some(resonator),
            config: ResonatorConfig::default(),
        }
    }

    /// Create from a codebook (vec of symbols).
    ///
    /// # Errors
    ///
    /// Returns error if the codebook is empty.
    pub fn from_symbols(symbols: Vec<A>) -> MinuetResult<Self> {
        if symbols.is_empty() {
            return Err(MinuetError::config("Codebook cannot be empty"));
        }

        let config = ResonatorConfig::default();
        let resonator = Resonator::new(symbols, config.clone()).map_err(MinuetError::algebra)?;

        Ok(Self {
            resonator: Some(resonator),
            config,
        })
    }

    /// Set configuration.
    #[must_use]
    pub fn with_config(mut self, config: ResonatorConfig) -> Self {
        self.config = config;
        self
    }

    /// Set initial temperature (low = soft attention).
    #[must_use]
    pub fn initial_temperature(mut self, temp: f64) -> Self {
        self.config.initial_beta = temp;
        self
    }

    /// Set final temperature (high = hard selection).
    #[must_use]
    pub fn final_temperature(mut self, temp: f64) -> Self {
        self.config.final_beta = temp;
        self
    }

    /// Set max iterations.
    #[must_use]
    pub fn max_iterations(mut self, iters: usize) -> Self {
        self.config.max_iterations = iters;
        self
    }

    /// Configure cleanup annealing from a [`Temperature`](super::Temperature).
    ///
    /// This is the WS 4 seam that closes the loop between the restored
    /// annealed-temperature API and the resonator cleanup loop. A
    /// [`Temperature::annealed`](super::Temperature::annealed) value drives
    /// the resonator from a soft (broad basin) start toward a hard
    /// (tropical/winner-take-all) finish across the configured steps —
    /// exactly the behaviour a noisy microwave backend needs to reject
    /// phase noise during cleanup.
    ///
    /// Additive over the raw-beta setters
    /// ([`initial_temperature`](Self::initial_temperature) /
    /// [`final_temperature`](Self::final_temperature)): this overwrites both
    /// betas (and `max_iterations` for an annealed temperature) from the
    /// single `Temperature` value. See
    /// [`Temperature::to_resonator_config`](super::Temperature::to_resonator_config)
    /// for the exact mapping.
    ///
    /// If this retriever already holds a built resonator, it is discarded so
    /// the next [`cleanup`](Retriever::cleanup) rebuilds from the codebook
    /// using the new config (a pre-built resonator bakes the old config in).
    #[must_use]
    pub fn with_temperature(mut self, temperature: &super::Temperature) -> Self {
        self.config = temperature.to_resonator_config();
        self.resonator = None;
        self
    }

    /// Configure cleanup annealing from a [`TemperatureSchedule`](super::TemperatureSchedule).
    ///
    /// Uses the schedule's first and last beta as the resonator's
    /// `initial_beta` / `final_beta` and the schedule length as
    /// `max_iterations`. The full schedule shape (linear / exponential /
    /// cosine) is approximated by the resonator's own two-point anneal —
    /// amari's `ResonatorConfig` exposes only start/end betas, so a richer
    /// per-iteration shape is not yet expressible through this seam.
    ///
    /// An empty schedule is a no-op (the existing config is kept).
    #[must_use]
    pub fn with_temperature_schedule(mut self, schedule: &super::TemperatureSchedule) -> Self {
        if let (Some(&first), Some(&last)) = (
            schedule.temperatures().first(),
            schedule.temperatures().last(),
        ) {
            self.config.initial_beta = first;
            self.config.final_beta = last;
            self.config.max_iterations = schedule.len();
            self.resonator = None;
        }
        self
    }
}

impl<A: BindingAlgebra> Retriever for ResonatorRetriever<A> {
    type Algebra = A;

    fn cleanup(&self, raw: &A, context: &RetrievalContext<A>) -> MinuetResult<CleanupResult<A>> {
        // Try to use pre-built resonator, or build from context codebook
        let result = if let Some(ref resonator) = self.resonator {
            resonator.cleanup(raw)
        } else if let Some(ref codebook) = context.codebook {
            if codebook.is_empty() {
                return Ok(CleanupResult {
                    value: raw.clone(),
                    confidence: 0.5,
                    iterations: 0,
                    converged: false,
                    codebook_match: None,
                });
            }

            let temp_resonator = Resonator::new(codebook.clone(), self.config.clone())
                .map_err(MinuetError::algebra)?;
            temp_resonator.cleanup(raw)
        } else {
            // No codebook available, return raw
            return Ok(CleanupResult {
                value: raw.clone(),
                confidence: 0.5,
                iterations: 0,
                converged: false,
                codebook_match: None,
            });
        };

        Ok(CleanupResult {
            value: result.cleaned,
            confidence: result.final_similarity,
            iterations: result.iterations,
            converged: result.converged,
            codebook_match: Some(result.best_match_index),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retrieval::{Temperature, TemperatureSchedule};
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>;

    #[test]
    fn cleanup_with_codebook() {
        // Create some symbols
        let symbols: Vec<TestAlgebra> = (0..5).map(|_| TestAlgebra::random_versor(2)).collect();

        let retriever = ResonatorRetriever::from_symbols(symbols.clone()).unwrap();

        // Cleanup should converge to one of the symbols
        let context = RetrievalContext::default();
        let result = retriever.cleanup(&symbols[2], &context).unwrap();

        assert!(result.converged);
        assert!(result.confidence > 0.9);
        assert!(result.codebook_match.is_some());
    }

    #[test]
    fn cleanup_without_codebook_returns_raw() {
        let retriever = ResonatorRetriever::<TestAlgebra>::new();
        let raw = TestAlgebra::random_versor(2);
        let context = RetrievalContext::default();

        let result = retriever.cleanup(&raw, &context).unwrap();

        // Without codebook, should return raw
        assert!(result.value.similarity(&raw) > 0.99);
    }

    /// WS 4: an annealed `Temperature` drives cleanup from a soft start to a
    /// hard finish and still converges on the correct codebook symbol. Uses
    /// 50 steps to match the default config's proven-sufficient iteration
    /// budget (amari's resonator anneals one beta-step per iteration, so
    /// `Annealed{steps}` maps to `max_iterations = steps`).
    ///
    /// `with_temperature` clears any pre-built resonator (so the new config
    /// takes effect), so the codebook is supplied via the retrieval context.
    #[test]
    fn cleanup_with_annealed_temperature() {
        let symbols: Vec<TestAlgebra> = (0..5).map(|_| TestAlgebra::random_versor(2)).collect();

        let retriever = ResonatorRetriever::new()
            .with_temperature(&Temperature::annealed(1.0, 100.0, 50).unwrap());

        let context = RetrievalContext::default().with_codebook(symbols.clone());
        let result = retriever.cleanup(&symbols[2], &context).unwrap();

        assert!(result.converged);
        assert!(result.confidence > 0.9);
        assert_eq!(result.codebook_match, Some(2));
    }

    /// WS 4: `with_temperature` is additive and overwrites both betas (and
    /// max_iterations for an annealed temperature). The `from_symbols` path
    /// builds a resonator with the default config; applying a temperature
    /// afterward discards it and rebuilds, so the new config takes effect.
    #[test]
    fn with_temperature_overwrites_config() {
        let symbols: Vec<TestAlgebra> = (0..3).map(|_| TestAlgebra::random_versor(2)).collect();

        let retriever = ResonatorRetriever::from_symbols(symbols).unwrap();
        // Capture the default to prove the overwrite below.
        assert_eq!(retriever.config.max_iterations, 50);

        let retriever = retriever.with_temperature(&Temperature::annealed(2.0, 50.0, 12).unwrap());
        assert_eq!(retriever.config.initial_beta, 2.0);
        assert_eq!(retriever.config.final_beta, 50.0);
        assert_eq!(retriever.config.max_iterations, 12);
        // The pre-built resonator was discarded so the new config takes effect.
        assert!(retriever.resonator.is_none());
    }

    /// WS 4: a `TemperatureSchedule` maps its endpoints + length into the
    /// resonator config.
    #[test]
    fn with_temperature_schedule_maps_endpoints() {
        let retriever = ResonatorRetriever::<TestAlgebra>::new()
            .with_temperature_schedule(&TemperatureSchedule::cosine(1.0, 10.0, 8));
        assert!((retriever.config.initial_beta - 1.0).abs() < 1e-12);
        assert!((retriever.config.final_beta - 10.0).abs() < 1e-12);
        assert_eq!(retriever.config.max_iterations, 8);
    }

    /// WS 4: an empty schedule is a no-op (default config preserved).
    #[test]
    fn with_empty_schedule_is_noop() {
        let retriever = ResonatorRetriever::<TestAlgebra>::new()
            .with_temperature_schedule(&TemperatureSchedule::constant(1.0, 0));
        // Default ResonatorConfig::max_iterations is 50.
        assert_eq!(retriever.config.max_iterations, 50);
    }
}
