//! Persistence layer for holographic memory.
//!
//! This module provides:
//!
//! - **Snapshot**: Memory snapshots for serialization
//! - **Journal**: Append-only operation log for durability
//! - **Recovery**: Crash recovery from snapshots and journals
//!
//! # Persistence Strategy
//!
//! The trace is the complete state, making persistence straightforward:
//!
//! 1. **Snapshot**: Serialize the trace + metadata at checkpoints
//! 2. **Journal**: Append store operations for incremental durability
//! 3. **Recovery**: Load latest snapshot, replay journal entries
//!
//! # Example
//!
//! ```rust,ignore
//! use minuet::persistence::PersistentMemory;
//!
//! // Open or create a persistent memory
//! let memory = PersistentMemory::<f64, 8>::open("./memory.db")?;
//!
//! // Store with automatic journaling
//! memory.store(&key, &value)?;
//!
//! // Force a snapshot
//! memory.snapshot()?;
//!
//! // Recover from crash
//! let recovered = PersistentMemory::recover("./memory.db")?;
//! ```

mod journal;
mod recovery;
mod snapshot;

pub use journal::{Journal, JournalConfig, JournalEntry};
pub use recovery::{Recovery, RecoveryResult};
pub use snapshot::{Snapshot, SnapshotMetadata};

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{MinuetError, Result};
use crate::memory::{
    CapacityInfo, MemoryStore, MemoryTrace, MergeResult, Query, QueryResult, StoreReceipt,
};
use crate::precision::MinuetFloat;

use amari_fusion::{holographic::RetrievalResult, TropicalDualClifford};

/// A persistent holographic memory with journaling.
pub struct PersistentMemory<T: MinuetFloat, const DIM: usize> {
    /// In-memory trace.
    memory: MemoryTrace<T, DIM>,

    /// Journal for durability.
    journal: RwLock<Journal>,

    /// Path to snapshot file.
    snapshot_path: PathBuf,

    /// Operations between automatic snapshots.
    snapshot_interval: usize,

    /// Operations since last snapshot.
    operations_since_snapshot: AtomicUsize,
}

impl<T: MinuetFloat + Serialize + for<'de> Deserialize<'de>, const DIM: usize>
    PersistentMemory<T, DIM>
{
    /// Open or create a persistent memory at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Check for existing snapshot
        if path.exists() {
            Self::recover(path)
        } else {
            Ok(Self::new(path))
        }
    }

    /// Create a new persistent memory.
    fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();

        Self {
            memory: MemoryTrace::new().into_unknown(),
            journal: RwLock::new(Journal::new(path.with_extension("journal"))),
            snapshot_path: path.to_path_buf(),
            snapshot_interval: 1000,
            operations_since_snapshot: AtomicUsize::new(0),
        }
    }

    /// Recover from crash.
    pub fn recover(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Load snapshot
        let snapshot = Snapshot::load(path)?;
        let memory = snapshot.into_trace()?;

        // Replay journal
        let journal_path = path.with_extension("journal");
        let journal = if journal_path.exists() {
            let j = Journal::open(&journal_path)?;
            // Replay entries not in snapshot
            // (In full implementation, would apply journal entries)
            j
        } else {
            Journal::new(&journal_path)
        };

        Ok(Self {
            memory,
            journal: RwLock::new(journal),
            snapshot_path: path.to_path_buf(),
            snapshot_interval: 1000,
            operations_since_snapshot: AtomicUsize::new(0),
        })
    }

    /// Force a snapshot.
    pub fn snapshot(&self) -> Result<()> {
        let snapshot = Snapshot::from_trace(&self.memory)?;
        snapshot.save(&self.snapshot_path)?;
        self.operations_since_snapshot.store(0, Ordering::SeqCst);

        // Truncate journal after successful snapshot
        self.journal.write().truncate()?;

        Ok(())
    }

    /// Set the snapshot interval.
    pub fn set_snapshot_interval(&mut self, interval: usize) {
        self.snapshot_interval = interval;
    }

    /// Check if a snapshot is due.
    fn maybe_snapshot(&self) -> Result<()> {
        let ops = self
            .operations_since_snapshot
            .fetch_add(1, Ordering::SeqCst);

        if ops >= self.snapshot_interval {
            self.snapshot()?;
        }

        Ok(())
    }
}

impl<T: MinuetFloat + Serialize + for<'de> Deserialize<'de>, const DIM: usize> MemoryStore<T, DIM>
    for PersistentMemory<T, DIM>
where
    T: Send + Sync,
    TropicalDualClifford<T, DIM>: Send + Sync,
{
    fn store(
        &self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<StoreReceipt> {
        // Write to journal first (durability)
        {
            let mut journal = self.journal.write();
            journal.append_store(key, value)?;
        }

        // Then to memory
        let receipt = self.memory.store(key, value)?;

        // Check for snapshot
        self.maybe_snapshot()?;

        Ok(receipt)
    }

    fn store_batch(
        &self,
        pairs: &[(TropicalDualClifford<T, DIM>, TropicalDualClifford<T, DIM>)],
    ) -> Result<Vec<StoreReceipt>> {
        pairs.iter().map(|(k, v)| self.store(k, v)).collect()
    }

    fn retrieve(&self, key: &TropicalDualClifford<T, DIM>) -> Result<RetrievalResult<T, DIM>> {
        let value = self.memory.retrieve(key);
        let info = self.memory.capacity_info();

        Ok(RetrievalResult {
            value: value.clone(),
            raw_value: value,
            confidence: info.estimated_snr,
            attribution: Vec::new(),
            query_similarity: 1.0,
        })
    }

    fn query(&self, query: Query<T, DIM>) -> Result<QueryResult<T, DIM>> {
        query.execute(&self.memory)
    }

    fn capacity(&self) -> CapacityInfo {
        self.memory.capacity_info()
    }

    fn merge(&self, _other: &dyn MemoryStore<T, DIM>) -> Result<MergeResult> {
        Err(MinuetError::MergeFailed(
            "Merge not yet implemented for persistent memory".into(),
        ))
    }

    fn trace(&self) -> TropicalDualClifford<T, DIM> {
        self.memory.raw_trace()
    }

    fn clear(&self) -> Result<()> {
        self.memory.clear();
        self.journal.write().truncate()?;
        self.operations_since_snapshot.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn len(&self) -> usize {
        self.memory.item_count() as usize
    }
}
