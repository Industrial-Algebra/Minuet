//! T-matrix fingerprinting for hardware state validation.
//!
//! Fast validation of optical hardware state without storing or recomputing
//! the full transmission matrix. Uses probe patterns to detect:
//! - Same hardware with stable T-matrix
//! - Same hardware with drifted T-matrix (e.g., temperature change)
//! - Different hardware entirely

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::hardware::{HardwareError, OpticalHardware};
use super::now_timestamp;

/// Probe pattern for T-matrix fingerprinting.
///
/// Probes are deterministically generated from seeds, enabling
/// regeneration without storage of the full pattern.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbePattern {
    /// Seed used to generate this probe (for regeneration).
    pub seed: u64,
    /// Hash of the pattern (for quick comparison).
    pub pattern_hash: u64,
}

impl ProbePattern {
    /// Generate a probe pattern for given dimensions.
    pub fn generate(seed: u64, dimensions: (usize, usize)) -> Self {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        dimensions.hash(&mut hasher);

        Self {
            seed,
            pattern_hash: hasher.finish(),
        }
    }
}

/// Response to a probe pattern (what the hardware measured).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeResponse {
    /// Which probe this responds to.
    pub probe: ProbePattern,
    /// Measured output mode amplitudes.
    pub amplitudes: Vec<f32>,
    /// Measured total intensity.
    pub total_intensity: f32,
}

/// Compact T-matrix characterization for change detection.
///
/// A fingerprint captures the essential behavior of the optical system
/// using a small number of probe patterns. This enables fast validation
/// of hardware state without full T-matrix computation.
///
/// # Validation Thresholds
///
/// - Correlation > 0.95: Valid (T-matrix unchanged)
/// - Correlation 0.70-0.95: Drifted (recalibration may help)
/// - Correlation < 0.70: Different hardware
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TMatrixFingerprint {
    /// Responses to probe patterns.
    pub responses: Vec<ProbeResponse>,

    /// Hardware identifier (serial number, etc.).
    pub hardware_id: String,

    /// Temperature at characterization time (Celsius).
    pub temperature_celsius: f32,

    /// Capture timestamp (millis since epoch).
    pub captured_at: u64,

    /// Number of optical modes in the system.
    pub n_modes: usize,
}

/// Result of validating current hardware against fingerprint.
#[derive(Clone, Debug)]
pub enum FingerprintValidation {
    /// Hardware matches, T-matrix likely unchanged.
    Valid,

    /// Same hardware but T has drifted (e.g., temperature change).
    Drifted {
        /// Mean correlation between stored and current responses.
        correlation: f32,
        /// Estimated drift magnitude (1 - correlation).
        estimated_drift: f32,
    },

    /// Different hardware entirely.
    DifferentHardware {
        /// Expected hardware ID from fingerprint.
        expected_id: String,
        /// Actual hardware ID.
        actual_id: String,
    },

    /// No fingerprint available for comparison.
    NoFingerprint,
}

impl FingerprintValidation {
    /// Check if validation indicates hardware is usable.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid | Self::Drifted { .. })
    }

    /// Check if full recalibration is recommended.
    pub fn needs_full_calibration(&self) -> bool {
        matches!(self, Self::DifferentHardware { .. } | Self::NoFingerprint)
    }
}

impl TMatrixFingerprint {
    /// Default number of probes to use.
    pub const DEFAULT_N_PROBES: usize = 5;

    /// Threshold for considering T-matrix valid (unchanged).
    pub const VALID_THRESHOLD: f32 = 0.95;

    /// Threshold below which hardware is considered different.
    pub const DIFFERENT_THRESHOLD: f32 = 0.70;

    /// Capture fingerprint from current hardware.
    ///
    /// This displays probe patterns and measures responses to characterize
    /// the current T-matrix behavior.
    pub fn capture<H: OpticalHardware>(
        hardware: &mut H,
        n_probes: usize,
    ) -> Result<Self, HardwareError> {
        let mut responses = Vec::with_capacity(n_probes);
        let dimensions = hardware.dimensions();

        for i in 0..n_probes {
            // Deterministic probe seeds
            let seed = i as u64 * 12345 + 67890;
            let probe = ProbePattern::generate(seed, dimensions);

            // Generate and display probe pattern
            let hologram = Self::generate_probe_hologram(seed, dimensions);
            hardware.display(&hologram)?;

            // Measure response
            let measurement = hardware.measure()?;

            responses.push(ProbeResponse {
                probe,
                amplitudes: measurement.mode_amplitudes,
                total_intensity: measurement.total_intensity,
            });
        }

        Ok(Self {
            responses,
            hardware_id: hardware.id().to_string(),
            temperature_celsius: hardware.temperature()?,
            captured_at: now_timestamp(),
            n_modes: hardware.n_modes(),
        })
    }

    /// Generate a probe hologram from seed.
    fn generate_probe_hologram(
        seed: u64,
        dimensions: (usize, usize),
    ) -> amari_holographic::optical::BinaryHologram {
        use amari_holographic::optical::{
            GeometricLeeEncoder, LeeEncoderConfig, OpticalRotorField,
        };

        let field = OpticalRotorField::random(dimensions, seed);
        let config = LeeEncoderConfig {
            carrier_frequency: 0.25,
            carrier_angle: 0.0,
            dimensions,
        };
        let encoder = GeometricLeeEncoder::new(config);
        encoder.encode(&field)
    }

    /// Validate current hardware against this fingerprint.
    pub fn validate<H: OpticalHardware>(
        &self,
        hardware: &mut H,
    ) -> Result<FingerprintValidation, HardwareError> {
        // Check hardware ID first
        if hardware.id() != self.hardware_id {
            return Ok(FingerprintValidation::DifferentHardware {
                expected_id: self.hardware_id.clone(),
                actual_id: hardware.id().to_string(),
            });
        }

        // Re-probe and compare
        let mut correlations = Vec::new();

        for stored in &self.responses {
            let hologram = Self::generate_probe_hologram(stored.probe.seed, hardware.dimensions());
            hardware.display(&hologram)?;

            let measurement = hardware.measure()?;

            // Compute correlation between stored and current response
            let corr = Self::correlation(&stored.amplitudes, &measurement.mode_amplitudes);
            correlations.push(corr);
        }

        let mean_correlation = if correlations.is_empty() {
            0.0
        } else {
            correlations.iter().sum::<f32>() / correlations.len() as f32
        };

        if mean_correlation > Self::VALID_THRESHOLD {
            Ok(FingerprintValidation::Valid)
        } else if mean_correlation > Self::DIFFERENT_THRESHOLD {
            Ok(FingerprintValidation::Drifted {
                correlation: mean_correlation,
                estimated_drift: 1.0 - mean_correlation,
            })
        } else {
            // Correlation too low - treat as different hardware even if ID matches
            // (fiber may have been replaced)
            Ok(FingerprintValidation::DifferentHardware {
                expected_id: self.hardware_id.clone(),
                actual_id: format!("{} (drifted beyond recognition)", hardware.id()),
            })
        }
    }

    /// Compute Pearson correlation coefficient between two vectors.
    fn correlation(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let n = a.len() as f32;
        let mean_a: f32 = a.iter().sum::<f32>() / n;
        let mean_b: f32 = b.iter().sum::<f32>() / n;

        let mut cov = 0.0f32;
        let mut var_a = 0.0f32;
        let mut var_b = 0.0f32;

        for (ai, bi) in a.iter().zip(b.iter()) {
            let da = ai - mean_a;
            let db = bi - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }

        let denom = (var_a * var_b).sqrt();
        if denom > 1e-10 {
            cov / denom
        } else {
            0.0
        }
    }

    /// Get the age of this fingerprint in seconds.
    pub fn age_seconds(&self) -> u64 {
        let now = now_timestamp();
        if now > self.captured_at {
            (now - self.captured_at) / 1000
        } else {
            0
        }
    }

    /// Check if fingerprint is older than given duration.
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        self.age_seconds() > max_age_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_pattern_generate() {
        let p1 = ProbePattern::generate(42, (64, 64));
        let p2 = ProbePattern::generate(42, (64, 64));
        let p3 = ProbePattern::generate(43, (64, 64));

        // Same seed and dimensions should produce same pattern
        assert_eq!(p1.seed, p2.seed);
        assert_eq!(p1.pattern_hash, p2.pattern_hash);

        // Different seed should produce different pattern
        assert_ne!(p1.pattern_hash, p3.pattern_hash);
    }

    #[test]
    fn test_correlation_identical() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let corr = TMatrixFingerprint::correlation(&a, &b);
        assert!((corr - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_correlation_opposite() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![5.0, 4.0, 3.0, 2.0, 1.0];

        let corr = TMatrixFingerprint::correlation(&a, &b);
        assert!((corr - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_correlation_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];

        let corr = TMatrixFingerprint::correlation(&a, &b);
        assert_eq!(corr, 0.0);
    }

    #[test]
    fn test_fingerprint_validation_usable() {
        assert!(FingerprintValidation::Valid.is_usable());
        assert!(FingerprintValidation::Drifted {
            correlation: 0.8,
            estimated_drift: 0.2
        }
        .is_usable());
        assert!(!FingerprintValidation::DifferentHardware {
            expected_id: "a".to_string(),
            actual_id: "b".to_string()
        }
        .is_usable());
    }
}
