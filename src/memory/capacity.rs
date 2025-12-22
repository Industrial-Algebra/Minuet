//! Capacity tracking and management for holographic memory.
//!
//! Holographic memory has limited capacity that scales as O(DIM / log DIM).
//! This module provides tools for tracking utilization and predicting
//! when capacity limits will be reached.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

#[cfg(feature = "contracts")]
use creusot_contracts::*;

use crate::error::CapacityWarning;
use crate::precision::MinuetFloat;

/// Capacity information for a memory trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityInfo {
    /// Current number of items stored.
    pub item_count: u64,

    /// Theoretical maximum capacity at 70% retrieval accuracy.
    pub theoretical_capacity: usize,

    /// Estimated signal-to-noise ratio.
    pub estimated_snr: f64,

    /// SNR threshold below which retrieval degrades unacceptably.
    pub snr_threshold: f64,

    /// Estimated remaining stores before threshold.
    pub remaining_stores: Option<usize>,

    /// Utilization as fraction of theoretical capacity.
    pub utilization: f64,
}

impl CapacityInfo {
    /// Check if memory is approaching capacity.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.utilization > 0.7
    }

    /// Check if memory is at capacity.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.estimated_snr <= self.snr_threshold
    }
}

/// Tracks capacity and SNR for a holographic trace.
#[derive(Debug)]
pub struct CapacityTracker<T> {
    /// Dimension of the representations.
    dim: usize,

    /// Theoretical capacity.
    theoretical_capacity: usize,

    /// Running sum of squared magnitudes (for SNR estimation).
    sum_sq_magnitudes: AtomicU64,

    /// Number of store operations.
    store_count: AtomicU64,

    /// SNR threshold for acceptable retrieval.
    snr_threshold: T,

    /// Warning threshold (fraction of capacity).
    warning_threshold: f64,
}

impl<T: MinuetFloat> CapacityTracker<T> {
    /// Create a new capacity tracker.
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension of the representation space
    #[must_use]
    pub fn new(dim: usize) -> Self {
        let theoretical_capacity = crate::dimensions::theoretical_capacity(dim);
        Self {
            dim,
            theoretical_capacity,
            sum_sq_magnitudes: AtomicU64::new(0),
            store_count: AtomicU64::new(0),
            snr_threshold: T::from_f64(0.5).unwrap(), // 50% SNR minimum
            warning_threshold: 0.7,
        }
    }

    /// Create with custom thresholds.
    #[must_use]
    pub fn with_thresholds(dim: usize, snr_threshold: T, warning_threshold: f64) -> Self {
        let mut tracker = Self::new(dim);
        tracker.snr_threshold = snr_threshold;
        tracker.warning_threshold = warning_threshold;
        tracker
    }

    /// Record a store operation.
    pub fn record_store(&self) {
        self.store_count.fetch_add(1, Ordering::SeqCst);
        // In a real implementation, we'd track actual magnitudes
        // For now, use unit magnitude assumption
        let bits = 1.0f64.to_bits();
        self.sum_sq_magnitudes.fetch_add(bits, Ordering::SeqCst);
    }

    /// Estimate SNR for a given item count.
    ///
    /// SNR decreases as 1/sqrt(n) where n is the number of items,
    /// normalized by algebra dimension (2^DIM, not DIM).
    #[cfg_attr(feature = "contracts", ensures(result >= T::zero()))]
    #[must_use]
    pub fn estimate_snr(&self, item_count: u64) -> T {
        if item_count == 0 {
            return T::infinity();
        }

        // SNR ≈ sqrt(algebra_dim / n), where algebra_dim = 2^DIM
        let algebra_dim = 1usize << self.dim;
        let dim_f = T::from_usize(algebra_dim).unwrap();
        let n_f = T::from_u64(item_count).unwrap();

        (dim_f / n_f).sqrt()
    }

    /// Get the SNR threshold.
    #[must_use]
    pub fn snr_threshold(&self) -> T {
        self.snr_threshold
    }

    /// Check for capacity warnings.
    #[must_use]
    pub fn check_warning(&self) -> Option<CapacityWarning> {
        let count = self.store_count.load(Ordering::SeqCst);
        let utilization = count as f64 / self.theoretical_capacity as f64;

        if utilization >= 1.0 {
            Some(CapacityWarning::Critical {
                snr: self.estimate_snr(count).to_f64().unwrap(),
            })
        } else if utilization >= self.warning_threshold {
            let remaining = ((1.0 - utilization) * self.theoretical_capacity as f64) as usize;
            Some(CapacityWarning::Approaching {
                utilization,
                remaining_stores: remaining,
            })
        } else {
            None
        }
    }

    /// Get full capacity information.
    #[must_use]
    pub fn info(&self, item_count: u64) -> CapacityInfo {
        let snr = self.estimate_snr(item_count);
        let utilization = item_count as f64 / self.theoretical_capacity as f64;

        // Estimate remaining stores based on SNR degradation model
        let remaining = if snr <= self.snr_threshold {
            Some(0)
        } else {
            // Solve for n where SNR(n) = threshold
            // sqrt(algebra_dim / n) = threshold
            // n = algebra_dim / threshold^2
            let algebra_dim = 1usize << self.dim;
            let threshold_sq = self.snr_threshold * self.snr_threshold;
            let max_items = T::from_usize(algebra_dim).unwrap() / threshold_sq;
            let remaining_f = max_items - T::from_u64(item_count).unwrap();
            if remaining_f > T::zero() {
                Some(remaining_f.to_usize().unwrap_or(0))
            } else {
                Some(0)
            }
        };

        CapacityInfo {
            item_count,
            theoretical_capacity: self.theoretical_capacity,
            estimated_snr: snr.to_f64().unwrap(),
            snr_threshold: self.snr_threshold.to_f64().unwrap(),
            remaining_stores: remaining,
            utilization,
        }
    }

    /// Reset the tracker.
    pub fn reset(&self) {
        self.sum_sq_magnitudes.store(0, Ordering::SeqCst);
        self.store_count.store(0, Ordering::SeqCst);
    }
}

impl<T: MinuetFloat> Clone for CapacityTracker<T> {
    fn clone(&self) -> Self {
        Self {
            dim: self.dim,
            theoretical_capacity: self.theoretical_capacity,
            sum_sq_magnitudes: AtomicU64::new(self.sum_sq_magnitudes.load(Ordering::SeqCst)),
            store_count: AtomicU64::new(self.store_count.load(Ordering::SeqCst)),
            snr_threshold: self.snr_threshold,
            warning_threshold: self.warning_threshold,
        }
    }
}

/// Capacity estimation utilities.
pub mod estimation {
    /// Estimate capacity at a target retrieval accuracy.
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension of representations
    /// * `target_accuracy` - Desired retrieval accuracy (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Estimated number of items that can be stored while maintaining accuracy.
    #[must_use]
    pub fn capacity_at_accuracy(dim: usize, target_accuracy: f64) -> usize {
        // Empirical model: accuracy ≈ 1 - k*n/DIM for some k
        // Solving for n: n = DIM * (1 - accuracy) / k
        // Using k ≈ 0.5 based on typical holographic memory behavior
        let k = 0.5;
        let max_degradation = 1.0 - target_accuracy;
        ((dim as f64) * max_degradation / k) as usize
    }

    /// Predict accuracy at a given load.
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension of representations
    /// * `item_count` - Number of items stored
    ///
    /// # Returns
    ///
    /// Predicted retrieval accuracy (0.0 to 1.0).
    #[must_use]
    pub fn accuracy_at_load(dim: usize, item_count: usize) -> f64 {
        let k = 0.5;
        (1.0 - k * (item_count as f64) / (dim as f64)).max(0.0)
    }

    /// Compute optimal dimension for a target capacity and accuracy.
    ///
    /// # Arguments
    ///
    /// * `target_capacity` - Desired number of items
    /// * `target_accuracy` - Desired retrieval accuracy
    ///
    /// # Returns
    ///
    /// Recommended dimension (rounded to nearest power of 2).
    #[must_use]
    pub fn optimal_dimension(target_capacity: usize, target_accuracy: f64) -> usize {
        let k = 0.5;
        let raw_dim = (target_capacity as f64) * k / (1.0 - target_accuracy);
        // Round up to nearest power of 2
        (raw_dim as usize).next_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snr_decreases_with_items() {
        // DIM=8 gives algebra_dim = 2^8 = 256 basis elements
        let tracker: CapacityTracker<f64> = CapacityTracker::new(8);

        let snr_1 = tracker.estimate_snr(1);
        let snr_10 = tracker.estimate_snr(10);
        let snr_100 = tracker.estimate_snr(100);

        assert!(snr_1 > snr_10);
        assert!(snr_10 > snr_100);

        // Verify expected SNR: sqrt(algebra_dim / n)
        // For n=1: sqrt(256/1) = 16
        assert!((snr_1 - 16.0).abs() < 0.1);
    }

    #[test]
    fn empty_has_infinite_snr() {
        let tracker: CapacityTracker<f64> = CapacityTracker::new(8);
        let snr = tracker.estimate_snr(0);
        assert!(snr.is_infinite());
    }

    #[test]
    fn capacity_estimation() {
        // At 80% accuracy, should be able to store more than at 90%
        // Note: estimation functions use algebra_dim (256 for DIM=8) as input
        let algebra_dim = 256; // 2^8
        let cap_80 = estimation::capacity_at_accuracy(algebra_dim, 0.8);
        let cap_90 = estimation::capacity_at_accuracy(algebra_dim, 0.9);
        assert!(cap_80 > cap_90);
    }

    #[test]
    fn optimal_dimension_scaling() {
        let dim_100 = estimation::optimal_dimension(100, 0.8);
        let dim_1000 = estimation::optimal_dimension(1000, 0.8);
        // 10x capacity should need roughly 10x dimension
        assert!(dim_1000 > dim_100 * 5);
    }
}
