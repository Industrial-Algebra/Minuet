//! Append-only journal for durability.

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::precision::MinuetFloat;

use amari_fusion::holographic::TropicalDualClifford;

/// Configuration for the journal.
#[derive(Debug, Clone)]
pub struct JournalConfig {
    /// Sync to disk after each write.
    pub sync_on_write: bool,

    /// Maximum journal size before forcing snapshot.
    pub max_size_bytes: usize,
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            sync_on_write: true,
            max_size_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// A journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEntry<T, const DIM: usize> {
    /// Store operation.
    Store {
        /// Sequence number.
        seq: u64,
        /// Key.
        key: TropicalDualClifford<T, DIM>,
        /// Value.
        value: TropicalDualClifford<T, DIM>,
    },
    /// Clear operation.
    Clear {
        /// Sequence number.
        seq: u64,
    },
    /// Checkpoint marker.
    Checkpoint {
        /// Sequence number.
        seq: u64,
        /// Snapshot ID this checkpoint corresponds to.
        snapshot_id: u64,
    },
}

/// Append-only operation journal.
pub struct Journal {
    /// Path to journal file.
    path: PathBuf,

    /// Current sequence number.
    sequence: u64,

    /// Configuration.
    config: JournalConfig,
}

impl Journal {
    /// Create a new journal.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            sequence: 0,
            config: JournalConfig::default(),
        }
    }

    /// Open an existing journal.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Read to find last sequence number
        let mut sequence = 0u64;

        if path.exists() {
            // In full implementation, would scan journal for last sequence
            // For now, just start fresh
        }

        Ok(Self {
            path: path.to_path_buf(),
            sequence,
            config: JournalConfig::default(),
        })
    }

    /// Append a store operation.
    pub fn append_store<T: MinuetFloat + Serialize, const DIM: usize>(
        &mut self,
        key: &TropicalDualClifford<T, DIM>,
        value: &TropicalDualClifford<T, DIM>,
    ) -> Result<()> {
        self.sequence += 1;

        let entry: JournalEntry<T, DIM> = JournalEntry::Store {
            seq: self.sequence,
            key: key.clone(),
            value: value.clone(),
        };

        self.append_entry(&entry)
    }

    /// Append a clear operation.
    pub fn append_clear<T: MinuetFloat + Serialize, const DIM: usize>(&mut self) -> Result<()> {
        self.sequence += 1;

        let entry: JournalEntry<T, DIM> = JournalEntry::Clear {
            seq: self.sequence,
        };

        self.append_entry(&entry)
    }

    /// Append a checkpoint marker.
    pub fn append_checkpoint<T: MinuetFloat + Serialize, const DIM: usize>(
        &mut self,
        snapshot_id: u64,
    ) -> Result<()> {
        self.sequence += 1;

        let entry: JournalEntry<T, DIM> = JournalEntry::Checkpoint {
            seq: self.sequence,
            snapshot_id,
        };

        self.append_entry(&entry)
    }

    /// Append an entry to the journal.
    fn append_entry<T: Serialize>(&mut self, entry: &T) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let mut writer = BufWriter::new(file);
        bincode::serialize_into(&mut writer, entry)?;

        if self.config.sync_on_write {
            writer.flush()?;
        }

        Ok(())
    }

    /// Truncate the journal (after successful snapshot).
    pub fn truncate(&mut self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Get the current sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get the path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
