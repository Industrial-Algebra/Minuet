//! Crash recovery for persistent memory.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::memory::MemoryTrace;
use crate::precision::MinuetFloat;

use super::journal::Journal;
use super::snapshot::Snapshot;

/// Result of a recovery operation.
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Number of entries recovered from journal.
    pub journal_entries_replayed: usize,

    /// Whether the snapshot was found and loaded.
    pub snapshot_loaded: bool,

    /// Final item count after recovery.
    pub final_item_count: u64,

    /// Any warnings during recovery.
    pub warnings: Vec<String>,
}

/// Recovery utilities for persistent memory.
pub struct Recovery;

impl Recovery {
    /// Recover a memory trace from snapshot and journal.
    pub fn recover<T, const DIM: usize>(
        snapshot_path: impl AsRef<Path>,
        journal_path: impl AsRef<Path>,
    ) -> Result<(MemoryTrace<T, DIM>, RecoveryResult)>
    where
        T: MinuetFloat + Serialize + for<'de> Deserialize<'de>,
    {
        let snapshot_path = snapshot_path.as_ref();
        let journal_path = journal_path.as_ref();

        let mut result = RecoveryResult {
            journal_entries_replayed: 0,
            snapshot_loaded: false,
            final_item_count: 0,
            warnings: Vec::new(),
        };

        // Try to load snapshot
        let trace = if snapshot_path.exists() {
            match Snapshot::<T, DIM>::load(snapshot_path) {
                Ok(snapshot) => {
                    result.snapshot_loaded = true;
                    snapshot.into_trace()?
                }
                Err(e) => {
                    result.warnings.push(format!("Failed to load snapshot: {}", e));
                    MemoryTrace::new().into_unknown()
                }
            }
        } else {
            MemoryTrace::new().into_unknown()
        };

        // Replay journal if exists
        if journal_path.exists() {
            // In full implementation, would replay journal entries
            // For now, just note that recovery would happen
            result.warnings.push("Journal replay not yet implemented".into());
        }

        result.final_item_count = trace.item_count();

        Ok((trace, result))
    }

    /// Check if recovery is needed.
    pub fn needs_recovery(
        snapshot_path: impl AsRef<Path>,
        journal_path: impl AsRef<Path>,
    ) -> bool {
        let journal_path = journal_path.as_ref();

        // Recovery is needed if journal exists and has entries
        if journal_path.exists() {
            if let Ok(metadata) = std::fs::metadata(journal_path) {
                return metadata.len() > 0;
            }
        }

        false
    }

    /// Validate a snapshot file.
    pub fn validate_snapshot<T, const DIM: usize>(path: impl AsRef<Path>) -> Result<bool>
    where
        T: MinuetFloat + for<'de> Deserialize<'de>,
    {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(false);
        }

        // Try to deserialize
        match Snapshot::<T, DIM>::load(path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_from_empty() {
        let (trace, result) =
            Recovery::recover::<f64, 64>("nonexistent.snapshot", "nonexistent.journal").unwrap();

        assert_eq!(trace.item_count(), 0);
        assert!(!result.snapshot_loaded);
    }
}
