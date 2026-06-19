// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware abstraction for optical computing backends.
//!
//! This module defines the `OpticalHardware` trait that abstracts over:
//! - Real hardware (DMD + MMF + camera)
//! - Simulated hardware (for testing)
//! - Remote hardware (network interface)

use std::collections::HashMap;

use amari_holographic::optical::BinaryHologram;

use super::now_timestamp;

/// Measurement result from optical hardware.
#[derive(Clone, Debug)]
pub struct OpticalMeasurement {
    /// Mode amplitudes (projected intensities for each mode).
    pub mode_amplitudes: Vec<f32>,
    /// Total integrated intensity across all modes.
    pub total_intensity: f32,
    /// Measurement timestamp (millis since epoch).
    pub timestamp: u64,
}

impl OpticalMeasurement {
    /// Create a new measurement with current timestamp.
    pub fn new(mode_amplitudes: Vec<f32>) -> Self {
        let total_intensity = mode_amplitudes.iter().sum();
        Self {
            mode_amplitudes,
            total_intensity,
            timestamp: now_timestamp(),
        }
    }

    /// Number of modes in this measurement.
    pub fn n_modes(&self) -> usize {
        self.mode_amplitudes.len()
    }

    /// Get normalized mode amplitudes (sum to 1).
    pub fn normalized_amplitudes(&self) -> Vec<f32> {
        if self.total_intensity > 0.0 {
            self.mode_amplitudes
                .iter()
                .map(|a| a / self.total_intensity)
                .collect()
        } else {
            vec![0.0; self.mode_amplitudes.len()]
        }
    }

    /// Find the index of the maximum amplitude mode.
    pub fn dominant_mode(&self) -> Option<usize> {
        self.mode_amplitudes
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }
}

/// Calibration state for optical hardware.
#[derive(Clone, Debug)]
pub struct HardwareCalibration {
    /// Cached inverse-design patterns (target mode hash → hologram).
    pub pattern_cache: HashMap<u64, BinaryHologram>,
    /// Temperature at calibration time (Celsius).
    pub calibration_temperature: f32,
    /// Calibration timestamp (millis since epoch).
    pub calibrated_at: u64,
}

impl HardwareCalibration {
    /// Create a new empty calibration.
    pub fn new(temperature: f32) -> Self {
        Self {
            pattern_cache: HashMap::new(),
            calibration_temperature: temperature,
            calibrated_at: now_timestamp(),
        }
    }

    /// Check if a pattern is cached.
    pub fn has_pattern(&self, hash: u64) -> bool {
        self.pattern_cache.contains_key(&hash)
    }

    /// Get a cached pattern.
    pub fn get_pattern(&self, hash: u64) -> Option<&BinaryHologram> {
        self.pattern_cache.get(&hash)
    }

    /// Cache a pattern.
    pub fn cache_pattern(&mut self, hash: u64, hologram: BinaryHologram) {
        self.pattern_cache.insert(hash, hologram);
    }

    /// Get the age of this calibration in seconds.
    pub fn age_seconds(&self) -> u64 {
        let now = now_timestamp();
        if now > self.calibrated_at {
            (now - self.calibrated_at) / 1000
        } else {
            0
        }
    }

    /// Check if calibration is stale.
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        self.age_seconds() > max_age_seconds
    }
}

/// Hardware error types.
#[derive(Debug)]
pub enum HardwareError {
    /// Communication error with hardware.
    Communication(String),
    /// Hardware not ready.
    NotReady,
    /// Calibration required before operation.
    CalibrationRequired,
    /// Measurement failed.
    MeasurementFailed(String),
    /// Pattern display failed.
    DisplayFailed(String),
    /// Temperature out of acceptable range.
    TemperatureOutOfRange {
        /// Current temperature.
        current: f32,
        /// Minimum acceptable temperature.
        min: f32,
        /// Maximum acceptable temperature.
        max: f32,
    },
    /// Hardware timeout.
    Timeout(String),
    /// Generic hardware error.
    Other(String),
}

impl std::fmt::Display for HardwareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Communication(msg) => write!(f, "hardware communication error: {msg}"),
            Self::NotReady => write!(f, "hardware not ready"),
            Self::CalibrationRequired => write!(f, "calibration required"),
            Self::MeasurementFailed(msg) => write!(f, "measurement failed: {msg}"),
            Self::DisplayFailed(msg) => write!(f, "pattern display failed: {msg}"),
            Self::TemperatureOutOfRange { current, min, max } => {
                write!(f, "temperature {current} out of range [{min}, {max}]")
            }
            Self::Timeout(msg) => write!(f, "hardware timeout: {msg}"),
            Self::Other(msg) => write!(f, "hardware error: {msg}"),
        }
    }
}

impl std::error::Error for HardwareError {}

/// Abstraction over optical hardware.
///
/// Implementations may be:
/// - Real hardware (DMD + MMF + camera)
/// - Simulated hardware (for testing)
/// - Remote hardware (network interface)
///
/// # Thread Safety
///
/// Implementations must be `Send` to allow use across threads.
/// Mutable access is required for display and measurement operations.
pub trait OpticalHardware: Send {
    /// Hardware identifier (serial number or unique name).
    fn id(&self) -> &str;

    /// Grid dimensions (matches DMD resolution).
    fn dimensions(&self) -> (usize, usize);

    /// Number of optical modes supported.
    fn n_modes(&self) -> usize;

    /// Current temperature in Celsius.
    fn temperature(&self) -> Result<f32, HardwareError>;

    /// Display a binary hologram pattern on the SLM/DMD.
    fn display(&mut self, hologram: &BinaryHologram) -> Result<(), HardwareError>;

    /// Measure current output from the optical system.
    fn measure(&mut self) -> Result<OpticalMeasurement, HardwareError>;

    /// Perform quick calibration (when T-matrix likely unchanged).
    ///
    /// This is faster than full calibration and suitable when:
    /// - Temperature hasn't changed significantly
    /// - Hardware hasn't been disturbed
    /// - Fingerprint validation passed
    fn quick_calibrate(&mut self) -> Result<HardwareCalibration, HardwareError>;

    /// Perform full calibration (new hardware or significant drift).
    ///
    /// This is slower but more thorough:
    /// - Characterizes full T-matrix behavior
    /// - Updates inverse-design patterns
    /// - Suitable after hardware changes
    fn full_calibrate(&mut self) -> Result<HardwareCalibration, HardwareError>;

    /// Check if hardware is ready for operations.
    fn is_ready(&self) -> bool;

    /// Reset hardware to initial state.
    fn reset(&mut self) -> Result<(), HardwareError> {
        // Default: no-op for hardware that doesn't need reset
        Ok(())
    }

    /// Get hardware-specific statistics or diagnostics.
    fn diagnostics(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}
