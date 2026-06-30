// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0
//! Temperature control for soft/hard retrieval interpolation.
//!
//! Temperature controls the sharpness of operations in holographic memory:
//! - Low temperature (high beta): sharp, tropical (max) operations
//! - High temperature (low beta): soft, smooth operations
//!
//! Restored in the Kagome-readiness sprint (WS 2) from the pre-v0.3.0 codebase.
//! Annealed [`Temperature`] and [`TemperatureSchedule`] schedules drive the
//! resonator-cleanup loop that a noisy microwave backend (Kagome) depends on:
//! they let cleanup start soft (broad basin) and sharpen toward tropical/max
//! across iterations.

use crate::error::{MinuetError, Result};

/// Temperature settings for retrieval operations.
///
/// Temperature controls the interpolation between soft (standard) and hard
/// (tropical/max) operations in the algebra.
///
/// # Examples
///
/// ```
/// use minuet::retrieval::Temperature;
///
/// // Explicit beta sharpens operations.
/// let sharp = Temperature::beta(10.0)?;
/// assert!((sharp.beta_at(0) - 10.0).abs() < 1e-12);
///
/// // Soft defaults to beta = 1 regardless of iteration.
/// assert!((Temperature::soft().beta_at(7) - 1.0).abs() < 1e-12);
/// # Ok::<(), minuet::error::MinuetError>(())
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Temperature {
    /// Soft retrieval (standard operations, beta = 1).
    #[default]
    Soft,

    /// Hard retrieval (tropical/max operations, beta -> infinity).
    Hard,

    /// Explicit temperature value (beta > 0).
    Beta(f64),

    /// Anneal from soft to hard over iterations.
    Annealed {
        /// Starting beta value.
        start: f64,
        /// Ending beta value.
        end: f64,
        /// Number of steps.
        steps: usize,
    },
}

impl Temperature {
    /// Create a soft temperature (beta = 1).
    #[must_use]
    pub fn soft() -> Self {
        Self::Soft
    }

    /// Create a hard temperature (tropical operations).
    #[must_use]
    pub fn hard() -> Self {
        Self::Hard
    }

    /// Create with explicit beta value.
    ///
    /// # Errors
    ///
    /// Returns [`MinuetError::InvalidTemperature`] if `beta` is not positive.
    pub fn beta(beta: f64) -> Result<Self> {
        if beta <= 0.0 {
            return Err(MinuetError::InvalidTemperature {
                beta,
                min: 0.0,
                max: f64::INFINITY,
            });
        }
        Ok(Self::Beta(beta))
    }

    /// Create an annealing schedule.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting beta value (typically low, e.g., 1.0)
    /// * `end` - Ending beta value (typically high, e.g., 100.0)
    /// * `steps` - Number of annealing steps
    ///
    /// # Errors
    ///
    /// Returns [`MinuetError::InvalidTemperature`] if `start` or `end` is not
    /// positive, or [`MinuetError::InvalidQuery`] if `steps` is zero.
    pub fn annealed(start: f64, end: f64, steps: usize) -> Result<Self> {
        if start <= 0.0 {
            return Err(MinuetError::InvalidTemperature {
                beta: start,
                min: 0.0,
                max: f64::INFINITY,
            });
        }
        if end <= 0.0 {
            return Err(MinuetError::InvalidTemperature {
                beta: end,
                min: 0.0,
                max: f64::INFINITY,
            });
        }
        if steps == 0 {
            return Err(MinuetError::InvalidQuery(
                "Annealing steps must be positive".into(),
            ));
        }

        Ok(Self::Annealed { start, end, steps })
    }

    /// Get the beta value for a specific iteration.
    ///
    /// For non-annealed temperatures, `iteration` is ignored. Annealed
    /// temperatures follow an exponential (geometric) interpolation in log-space
    /// from `start` to `end`, clamping to `end` once `iteration >= steps`.
    #[must_use]
    pub fn beta_at(&self, iteration: usize) -> f64 {
        match self {
            Self::Soft => 1.0,
            Self::Hard => f64::MAX, // Represents tropical limit
            Self::Beta(b) => *b,
            Self::Annealed { start, end, steps } => {
                if iteration >= *steps {
                    *end
                } else {
                    // Exponential annealing
                    let progress = iteration as f64 / *steps as f64;
                    let log_start = start.ln();
                    let log_end = end.ln();
                    (log_start + progress * (log_end - log_start)).exp()
                }
            }
        }
    }

    /// Check if this is a hard (tropical) temperature.
    #[must_use]
    pub fn is_hard(&self) -> bool {
        matches!(self, Self::Hard)
    }

    /// Check if this is annealed.
    #[must_use]
    pub fn is_annealed(&self) -> bool {
        matches!(self, Self::Annealed { .. })
    }

    /// Get the number of annealing steps (1 for non-annealed).
    #[must_use]
    pub fn num_steps(&self) -> usize {
        match self {
            Self::Annealed { steps, .. } => *steps,
            _ => 1,
        }
    }

    /// Translate this temperature into an amari-holographic
    /// [`ResonatorConfig`](amari_holographic::ResonatorConfig), preserving
    /// the default convergence threshold for non-annealed variants.
    ///
    /// The mapping is:
    /// - [`Temperature::Soft`] → `initial_beta = final_beta = 1.0` (no anneal).
    /// - [`Temperature::Hard`] → both betas very large (winner-take-all).
    /// - [`Temperature::Beta`](`Temperature::Beta`) → both betas `b` (fixed sharpness).
    /// - [`Temperature::Annealed { start, end, steps }`] →
    ///   `initial_beta = start`, `final_beta = end`, `max_iterations = steps`
    ///   (annealed cleanup, the value a noisy microwave backend relies on).
    ///
    /// This is the seam that lets a
    /// [`ResonatorRetriever`](crate::retrieval::ResonatorRetriever) be
    /// configured from a `Temperature` rather than raw beta floats.
    #[must_use]
    pub fn to_resonator_config(&self) -> amari_holographic::ResonatorConfig {
        // Beta used for the "hard" tropical limit. amari's default final_beta
        // is 100.0; we use f64::MAX as the symbolic tropical/winner-take-all
        // value (matches `Temperature::Hard`'s `beta_at` semantics).
        const HARD_BETA: f64 = f64::MAX;

        let mut config = amari_holographic::ResonatorConfig::default();
        match self {
            Self::Soft => {
                config.initial_beta = 1.0;
                config.final_beta = 1.0;
            }
            Self::Hard => {
                config.initial_beta = HARD_BETA;
                config.final_beta = HARD_BETA;
            }
            Self::Beta(b) => {
                config.initial_beta = *b;
                config.final_beta = *b;
            }
            Self::Annealed { start, end, steps } => {
                config.initial_beta = *start;
                config.final_beta = *end;
                config.max_iterations = *steps;
            }
        }
        config
    }
}

/// Temperature schedule for multi-step operations.
///
/// A precomputed sequence of beta values, iterable once. Built via the
/// [`constant`](TemperatureSchedule::constant),
/// [`linear`](TemperatureSchedule::linear),
/// [`exponential`](TemperatureSchedule::exponential), and
/// [`cosine`](TemperatureSchedule::cosine) constructors.
///
/// # Examples
///
/// ```
/// use minuet::retrieval::TemperatureSchedule;
///
/// let mut sched = TemperatureSchedule::linear(1.0, 10.0, 4);
/// let betas: Vec<f64> = sched.by_ref().collect();
/// assert_eq!(betas.len(), 4);
/// ```
#[derive(Debug, Clone)]
pub struct TemperatureSchedule {
    temperatures: Vec<f64>,
    current_step: usize,
}

impl TemperatureSchedule {
    /// Create a constant schedule.
    #[must_use]
    pub fn constant(beta: f64, steps: usize) -> Self {
        Self {
            temperatures: vec![beta; steps],
            current_step: 0,
        }
    }

    /// Create a linear annealing schedule.
    #[must_use]
    pub fn linear(start: f64, end: f64, steps: usize) -> Self {
        let temperatures: Vec<f64> = (0..steps)
            .map(|i| {
                let t = i as f64 / (steps - 1).max(1) as f64;
                start + t * (end - start)
            })
            .collect();

        Self {
            temperatures,
            current_step: 0,
        }
    }

    /// Create an exponential annealing schedule.
    #[must_use]
    pub fn exponential(start: f64, end: f64, steps: usize) -> Self {
        let log_start = start.ln();
        let log_end = end.ln();

        let temperatures: Vec<f64> = (0..steps)
            .map(|i| {
                let t = i as f64 / (steps - 1).max(1) as f64;
                (log_start + t * (log_end - log_start)).exp()
            })
            .collect();

        Self {
            temperatures,
            current_step: 0,
        }
    }

    /// Create a cosine annealing schedule.
    #[must_use]
    pub fn cosine(start: f64, end: f64, steps: usize) -> Self {
        use std::f64::consts::PI;

        let temperatures: Vec<f64> = (0..steps)
            .map(|i| {
                let t = i as f64 / (steps - 1).max(1) as f64;
                let cos_factor = (1.0 - (PI * t).cos()) / 2.0;
                start + cos_factor * (end - start)
            })
            .collect();

        Self {
            temperatures,
            current_step: 0,
        }
    }

    /// Get the current temperature.
    #[must_use]
    pub fn current(&self) -> f64 {
        self.temperatures
            .get(self.current_step)
            .copied()
            .unwrap_or(*self.temperatures.last().unwrap_or(&1.0))
    }

    /// Advance to the next step.
    pub fn step(&mut self) {
        if self.current_step < self.temperatures.len() {
            self.current_step += 1;
        }
    }

    /// Reset to the beginning.
    pub fn reset(&mut self) {
        self.current_step = 0;
    }

    /// Check if the schedule is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.temperatures.len()
    }

    /// Get all temperatures in the schedule.
    #[must_use]
    pub fn temperatures(&self) -> &[f64] {
        &self.temperatures
    }

    /// Number of steps in the schedule.
    #[must_use]
    pub fn len(&self) -> usize {
        self.temperatures.len()
    }

    /// Check if schedule is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.temperatures.is_empty()
    }
}

impl Iterator for TemperatureSchedule {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_step < self.temperatures.len() {
            let temp = self.temperatures[self.current_step];
            self.current_step += 1;
            Some(temp)
        } else {
            None
        }
    }
}

// NOTE: the pre-v0.3.0 `Temperature::beta_as<T: crate::precision::MinuetFloat>`
// helper is intentionally NOT restored here. Its sole dependency, the
// `crate::precision` module (and the `MinuetFloat` trait), was removed in the
// toolkit-conversion refactor and has no replacement in current Minuet.
// Callers needing a typed beta can convert directly, e.g.
// `temp.beta_at(i) as f32`. Revisit if a generic float-conversion trait is
// reintroduced.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_temperature() {
        let temp = Temperature::soft();
        assert!((temp.beta_at(0) - 1.0).abs() < 1e-10);
        assert!((temp.beta_at(100) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn explicit_beta() {
        let temp = Temperature::beta(2.5).unwrap();
        assert!((temp.beta_at(0) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn invalid_beta_rejected() {
        assert!(Temperature::beta(-1.0).is_err());
        assert!(Temperature::beta(0.0).is_err());
    }

    #[test]
    fn annealing_schedule() {
        let temp = Temperature::annealed(1.0, 100.0, 10).unwrap();

        let beta_0 = temp.beta_at(0);
        let beta_5 = temp.beta_at(5);
        let beta_9 = temp.beta_at(9);

        // Should increase monotonically
        assert!(beta_0 < beta_5);
        assert!(beta_5 < beta_9);

        // Endpoints should match
        assert!((beta_0 - 1.0).abs() < 0.1);
        assert!((temp.beta_at(10) - 100.0).abs() < 1.0);
    }

    #[test]
    fn schedule_iteration() {
        let mut schedule = TemperatureSchedule::exponential(1.0, 10.0, 5);

        let collected: Vec<f64> = schedule.by_ref().collect();
        assert_eq!(collected.len(), 5);

        // Should be monotonically increasing
        for window in collected.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn cosine_schedule() {
        let schedule = TemperatureSchedule::cosine(1.0, 10.0, 10);
        let temps = schedule.temperatures();

        // Endpoints
        assert!((temps[0] - 1.0).abs() < 1e-10);
        assert!((temps[9] - 10.0).abs() < 1e-10);

        // Cosine has slow start and end, fast middle
        let early_delta = temps[1] - temps[0];
        let mid_delta = temps[5] - temps[4];
        assert!(mid_delta > early_delta);
    }

    /// Verifies the additive `serde` feature-gating added during the WS 2
    /// restore round-trips. The pre-v0.3.0 derive was unconditional; it is now
    /// behind `#[cfg_attr(feature = "serde", derive(...))]`.
    #[cfg(feature = "serde")]
    #[test]
    fn temperature_serde_roundtrip() {
        let temps = vec![
            Temperature::Soft,
            Temperature::Hard,
            Temperature::Beta(2.5),
            Temperature::annealed(1.0, 100.0, 10).unwrap(),
        ];
        for original in temps {
            let encoded = bincode::serialize(&original).expect("serialize");
            let decoded: Temperature = bincode::deserialize(&encoded).expect("deserialize");
            assert_eq!(original.num_steps(), decoded.num_steps());
            // beta_at is a faithful proxy for enum equality across the variants.
            for i in 0..12 {
                assert!((original.beta_at(i) - decoded.beta_at(i)).abs() < 1e-9);
            }
        }
    }

    /// WS 4: `to_resonator_config` maps each `Temperature` variant into the
    /// amari `ResonatorConfig` betas (and iterations for the annealed case).
    #[test]
    fn to_resonator_config_mapping() {
        let default_max = amari_holographic::ResonatorConfig::default().max_iterations;

        let soft = Temperature::soft().to_resonator_config();
        assert_eq!(soft.initial_beta, 1.0);
        assert_eq!(soft.final_beta, 1.0);
        assert_eq!(soft.max_iterations, default_max);

        let hard = Temperature::hard().to_resonator_config();
        // `Hard` maps to `f64::MAX`, the same symbolic tropical-limit value
        // `Temperature::hard().beta_at(_)` returns.
        assert_eq!(hard.initial_beta, f64::MAX);
        assert_eq!(hard.final_beta, f64::MAX);

        let beta = Temperature::beta(3.0).unwrap().to_resonator_config();
        assert_eq!(beta.initial_beta, 3.0);
        assert_eq!(beta.final_beta, 3.0);

        let annealed = Temperature::annealed(2.0, 50.0, 17)
            .unwrap()
            .to_resonator_config();
        assert_eq!(annealed.initial_beta, 2.0);
        assert_eq!(annealed.final_beta, 50.0);
        assert_eq!(annealed.max_iterations, 17);
    }
}
