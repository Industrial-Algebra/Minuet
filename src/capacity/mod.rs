// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Capacity management.
//!
//! This module provides capacity monitoring, eviction policies, and
//! consolidation strategies for holographic memory.
//!
//! Currently provides basic implementations. More sophisticated policies
//! will be added in future versions.

use crate::traits::{CapacityInfo, CapacityPolicy};

/// A policy that rejects new items when at capacity.
#[derive(Clone, Debug, Default)]
pub struct RejectPolicy {
    /// Threshold for rejecting (utilization fraction).
    pub threshold: f64,
}

impl RejectPolicy {
    /// Create with default threshold (0.95).
    #[must_use]
    pub fn new() -> Self {
        Self { threshold: 0.95 }
    }

    /// Create with custom threshold.
    #[must_use]
    pub fn with_threshold(threshold: f64) -> Self {
        Self { threshold }
    }
}

impl CapacityPolicy for RejectPolicy {
    fn can_accept(&self, info: &CapacityInfo) -> bool {
        info.utilization < self.threshold
    }

    fn critical_threshold(&self) -> f64 {
        self.threshold
    }
}

/// A policy that always accepts new items (no capacity enforcement).
#[derive(Clone, Debug, Default)]
pub struct AcceptAllPolicy;

impl AcceptAllPolicy {
    /// Create a new accept-all policy.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl CapacityPolicy for AcceptAllPolicy {
    fn can_accept(&self, _info: &CapacityInfo) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::TraceCapacityInfo;

    fn make_info(utilization: f64) -> CapacityInfo {
        CapacityInfo {
            total_items: (utilization * 100.0) as usize,
            theoretical_capacity: 100,
            utilization,
            estimated_snr: 10.0 / (utilization + 0.1),
            per_trace: vec![TraceCapacityInfo {
                name: "test".into(),
                items: (utilization * 100.0) as usize,
                capacity: 100,
                utilization,
            }],
        }
    }

    #[test]
    fn reject_policy_below_threshold() {
        let policy = RejectPolicy::with_threshold(0.9);
        assert!(policy.can_accept(&make_info(0.5)));
        assert!(policy.can_accept(&make_info(0.89)));
    }

    #[test]
    fn reject_policy_above_threshold() {
        let policy = RejectPolicy::with_threshold(0.9);
        assert!(!policy.can_accept(&make_info(0.91)));
        assert!(!policy.can_accept(&make_info(0.99)));
    }

    #[test]
    fn accept_all_always_accepts() {
        let policy = AcceptAllPolicy::new();
        assert!(policy.can_accept(&make_info(0.0)));
        assert!(policy.can_accept(&make_info(0.99)));
        assert!(policy.can_accept(&make_info(1.5))); // Even over capacity
    }

    #[test]
    fn reject_policy_default_threshold() {
        let policy = RejectPolicy::new();
        // Default threshold is 0.95
        assert!(policy.can_accept(&make_info(0.94)));
        assert!(!policy.can_accept(&make_info(0.96)));
    }

    #[test]
    fn reject_policy_at_threshold() {
        let policy = RejectPolicy::with_threshold(0.5);
        // Exactly at threshold should reject (strict less-than)
        assert!(!policy.can_accept(&make_info(0.5)));
        assert!(policy.can_accept(&make_info(0.49)));
    }

    #[test]
    fn reject_policy_custom_thresholds() {
        let lenient = RejectPolicy::with_threshold(1.0);
        assert!(lenient.can_accept(&make_info(0.99)));

        let strict = RejectPolicy::with_threshold(0.1);
        assert!(!strict.can_accept(&make_info(0.2)));

        let moderate = RejectPolicy::with_threshold(0.5);
        assert!(moderate.can_accept(&make_info(0.3)));
        assert!(!moderate.can_accept(&make_info(0.6)));
    }

    #[test]
    fn capacity_info_empty() {
        let info = make_info(0.0);
        assert_eq!(info.total_items, 0);
        assert!(info.utilization == 0.0);
        assert!(info.estimated_snr > 1.0);
    }

    #[test]
    fn capacity_info_per_trace() {
        let info = make_info(0.5);
        assert_eq!(info.per_trace.len(), 1);
        assert_eq!(info.per_trace[0].name, "test");
        assert_eq!(info.per_trace[0].items, 50);
    }

    #[test]
    fn accept_all_policy_thresholds() {
        let policy = AcceptAllPolicy::new();
        // AcceptAll still returns default thresholds
        assert!(policy.warning_threshold() > 0.0);
        assert!(policy.critical_threshold() > 0.0);
        assert!(policy.critical_threshold() > policy.warning_threshold());
    }

    #[test]
    fn reject_policy_thresholds_match() {
        let policy = RejectPolicy::with_threshold(0.75);
        // Warning and critical can be adjusted but rejection is at threshold
        assert!(!policy.can_accept(&make_info(0.76)));
        assert!(policy.can_accept(&make_info(0.74)));
    }

    #[test]
    fn capacity_info_multi_trace() {
        let info = CapacityInfo {
            total_items: 20,
            theoretical_capacity: 200,
            utilization: 0.1,
            estimated_snr: 3.0,
            per_trace: vec![
                TraceCapacityInfo {
                    name: "shard_0".into(),
                    items: 10,
                    capacity: 100,
                    utilization: 0.1,
                },
                TraceCapacityInfo {
                    name: "shard_1".into(),
                    items: 10,
                    capacity: 100,
                    utilization: 0.1,
                },
            ],
        };
        assert_eq!(info.per_trace.len(), 2);
        assert_eq!(info.total_items, 20);
        assert_eq!(info.theoretical_capacity, 200);
    }
}
