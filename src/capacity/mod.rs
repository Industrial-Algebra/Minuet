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
}
