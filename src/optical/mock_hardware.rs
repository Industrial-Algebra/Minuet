// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Mock optical hardware for testing without physical devices.
//!
//! `MockOpticalHardware` simulates the behavior of a DMD + MMF optical system
//! with configurable properties for testing various scenarios.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use amari_holographic::optical::BinaryHologram;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::hardware::{HardwareCalibration, HardwareError, OpticalHardware, OpticalMeasurement};
use super::now_timestamp;

/// Mock optical hardware for testing without physical devices.
///
/// The mock simulates:
/// - A transmission matrix (T-matrix) mapping input modes to output modes
/// - Temperature-dependent behavior
/// - Pattern display and measurement
///
/// # Determinism
///
/// Given the same seed, the mock produces identical behavior, enabling
/// reproducible tests. Different seeds simulate different physical hardware.
///
/// # Example
///
/// ```ignore
/// use minuet::optical::MockOpticalHardware;
///
/// let mut hw = MockOpticalHardware::new(42);
/// assert!(hw.is_ready());
///
/// // Simulate T-matrix drift
/// hw.drift_t_matrix(0.1);
/// ```
pub struct MockOpticalHardware {
    /// Hardware identifier (derived from seed).
    id: String,
    /// Grid dimensions (simulated DMD resolution).
    dimensions: (usize, usize),
    /// Number of optical modes.
    n_modes: usize,
    /// Current temperature (Celsius).
    temperature: f32,
    /// Simulated T-matrix (n_modes × n_modes).
    t_matrix: Vec<Vec<f32>>,
    /// Currently displayed pattern.
    current_pattern: Option<BinaryHologram>,
    /// Random seed (same seed = same T-matrix).
    seed: u64,
    /// Whether hardware is ready.
    ready: bool,
}

impl MockOpticalHardware {
    /// Default grid dimensions (256×256).
    pub const DEFAULT_DIMENSIONS: (usize, usize) = (256, 256);

    /// Default number of optical modes.
    pub const DEFAULT_N_MODES: usize = 100;

    /// Default temperature (25°C).
    pub const DEFAULT_TEMPERATURE: f32 = 25.0;

    /// Create new mock hardware with given seed.
    ///
    /// The seed determines the T-matrix and hardware ID. Same seed
    /// produces identical hardware behavior.
    pub fn new(seed: u64) -> Self {
        Self::with_config(seed, Self::DEFAULT_DIMENSIONS, Self::DEFAULT_N_MODES)
    }

    /// Create mock hardware with custom configuration.
    pub fn with_config(seed: u64, dimensions: (usize, usize), n_modes: usize) -> Self {
        Self {
            id: format!("mock-{seed:016x}"),
            dimensions,
            n_modes,
            temperature: Self::DEFAULT_TEMPERATURE,
            t_matrix: Self::generate_t_matrix(n_modes, seed),
            current_pattern: None,
            seed,
            ready: true,
        }
    }

    /// Generate a simulated T-matrix.
    ///
    /// Creates a random unitary-ish matrix representing the fiber's
    /// mode coupling behavior.
    fn generate_t_matrix(n_modes: usize, seed: u64) -> Vec<Vec<f32>> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        (0..n_modes)
            .map(|_| {
                let row: Vec<f32> = (0..n_modes).map(|_| rng.gen_range(-1.0..1.0)).collect();
                // Normalize row
                let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    row.into_iter().map(|x| x / norm).collect()
                } else {
                    row
                }
            })
            .collect()
    }

    /// Simulate T-matrix drift (for testing fingerprint detection).
    ///
    /// Adds random perturbations to the T-matrix, simulating effects like
    /// temperature changes or fiber movement.
    ///
    /// # Arguments
    ///
    /// * `amount` - Magnitude of drift (0.0 to 1.0). Larger values cause
    ///   more significant changes.
    pub fn drift_t_matrix(&mut self, amount: f32) {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed.wrapping_add(999));

        for row in &mut self.t_matrix {
            for val in row.iter_mut() {
                *val += rng.gen_range(-amount..amount);
            }
            // Re-normalize
            let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for val in row.iter_mut() {
                    *val /= norm;
                }
            }
        }
    }

    /// Set the simulated temperature.
    pub fn set_temperature(&mut self, temp: f32) {
        self.temperature = temp;
    }

    /// Set whether hardware is ready.
    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    /// Get the seed used to create this hardware.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Simulate measurement through T-matrix.
    fn simulate_measurement(&self, pattern: &BinaryHologram) -> Vec<f32> {
        // Hash the pattern to get deterministic "input modes"
        let pattern_hash = {
            let mut h = DefaultHasher::new();
            pattern.as_bytes().hash(&mut h);
            h.finish()
        };

        let mut rng = ChaCha8Rng::seed_from_u64(pattern_hash);

        // Generate input mode amplitudes from pattern
        let input_modes: Vec<f32> = (0..self.n_modes).map(|_| rng.gen_range(0.0..1.0)).collect();

        // Apply T-matrix
        self.t_matrix
            .iter()
            .map(|row| {
                row.iter()
                    .zip(&input_modes)
                    .map(|(t, i)| t * i)
                    .sum::<f32>()
                    .abs()
            })
            .collect()
    }
}

impl OpticalHardware for MockOpticalHardware {
    fn id(&self) -> &str {
        &self.id
    }

    fn dimensions(&self) -> (usize, usize) {
        self.dimensions
    }

    fn n_modes(&self) -> usize {
        self.n_modes
    }

    fn temperature(&self) -> Result<f32, HardwareError> {
        Ok(self.temperature)
    }

    fn display(&mut self, hologram: &BinaryHologram) -> Result<(), HardwareError> {
        if !self.ready {
            return Err(HardwareError::NotReady);
        }

        if hologram.dimensions() != self.dimensions {
            return Err(HardwareError::DisplayFailed(format!(
                "dimension mismatch: expected {:?}, got {:?}",
                self.dimensions,
                hologram.dimensions()
            )));
        }

        self.current_pattern = Some(hologram.clone());
        Ok(())
    }

    fn measure(&mut self) -> Result<OpticalMeasurement, HardwareError> {
        if !self.ready {
            return Err(HardwareError::NotReady);
        }

        let pattern = self
            .current_pattern
            .as_ref()
            .ok_or_else(|| HardwareError::MeasurementFailed("no pattern displayed".to_string()))?;

        let output_modes = self.simulate_measurement(pattern);
        let total_intensity = output_modes.iter().sum();

        Ok(OpticalMeasurement {
            mode_amplitudes: output_modes,
            total_intensity,
            timestamp: now_timestamp(),
        })
    }

    fn quick_calibrate(&mut self) -> Result<HardwareCalibration, HardwareError> {
        if !self.ready {
            return Err(HardwareError::NotReady);
        }

        Ok(HardwareCalibration::new(self.temperature))
    }

    fn full_calibrate(&mut self) -> Result<HardwareCalibration, HardwareError> {
        // In mock, same as quick calibration
        self.quick_calibrate()
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    fn reset(&mut self) -> Result<(), HardwareError> {
        self.current_pattern = None;
        Ok(())
    }

    fn diagnostics(&self) -> std::collections::HashMap<String, String> {
        let mut diag = std::collections::HashMap::new();
        diag.insert("type".to_string(), "mock".to_string());
        diag.insert("seed".to_string(), format!("{}", self.seed));
        diag.insert(
            "dimensions".to_string(),
            format!("{}x{}", self.dimensions.0, self.dimensions.1),
        );
        diag.insert("n_modes".to_string(), format!("{}", self.n_modes));
        diag.insert(
            "temperature".to_string(),
            format!("{:.1}°C", self.temperature),
        );
        diag.insert(
            "pattern_loaded".to_string(),
            format!("{}", self.current_pattern.is_some()),
        );
        diag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_hardware_creation() {
        let hw = MockOpticalHardware::new(42);
        assert!(hw.is_ready());
        assert_eq!(hw.id(), "mock-000000000000002a");
        assert_eq!(hw.dimensions(), (256, 256));
        assert_eq!(hw.n_modes(), 100);
    }

    #[test]
    fn test_mock_hardware_determinism() {
        let hw1 = MockOpticalHardware::new(42);
        let hw2 = MockOpticalHardware::new(42);

        // Same seed should produce same T-matrix
        assert_eq!(hw1.t_matrix, hw2.t_matrix);
        assert_eq!(hw1.id(), hw2.id());
    }

    #[test]
    fn test_mock_hardware_different_seeds() {
        let hw1 = MockOpticalHardware::new(42);
        let hw2 = MockOpticalHardware::new(43);

        // Different seeds should produce different T-matrices
        assert_ne!(hw1.t_matrix, hw2.t_matrix);
        assert_ne!(hw1.id(), hw2.id());
    }

    #[test]
    fn test_display_and_measure() {
        let mut hw = MockOpticalHardware::new(42);

        let hologram = BinaryHologram::zeros((256, 256));
        hw.display(&hologram).unwrap();

        let measurement = hw.measure().unwrap();
        assert_eq!(measurement.n_modes(), 100);
        assert!(measurement.total_intensity >= 0.0);
    }

    #[test]
    fn test_display_dimension_mismatch() {
        let mut hw = MockOpticalHardware::new(42);

        let hologram = BinaryHologram::zeros((128, 128)); // Wrong size
        let result = hw.display(&hologram);

        assert!(matches!(result, Err(HardwareError::DisplayFailed(_))));
    }

    #[test]
    fn test_measure_without_pattern() {
        let mut hw = MockOpticalHardware::new(42);

        let result = hw.measure();
        assert!(matches!(result, Err(HardwareError::MeasurementFailed(_))));
    }

    #[test]
    fn test_not_ready() {
        let mut hw = MockOpticalHardware::new(42);
        hw.set_ready(false);

        let hologram = BinaryHologram::zeros((256, 256));
        assert!(matches!(
            hw.display(&hologram),
            Err(HardwareError::NotReady)
        ));
    }

    #[test]
    fn test_drift_t_matrix() {
        let mut hw = MockOpticalHardware::new(42);
        let original_t = hw.t_matrix.clone();

        hw.drift_t_matrix(0.1);

        // T-matrix should have changed
        assert_ne!(hw.t_matrix, original_t);
    }

    #[test]
    fn test_calibration() {
        let mut hw = MockOpticalHardware::new(42);

        let cal = hw.quick_calibrate().unwrap();
        assert!((cal.calibration_temperature - 25.0).abs() < 0.001);
    }
}
