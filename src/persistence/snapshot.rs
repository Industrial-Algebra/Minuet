//! Memory snapshot serialization.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::memory::MemoryTrace;
use crate::precision::MinuetFloat;

use amari_fusion::holographic::TropicalDualClifford;

/// Metadata about a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Creation timestamp.
    pub created_at: SystemTime,

    /// Number of items in the trace.
    pub item_count: u64,

    /// Dimension of representations.
    pub dimension: usize,

    /// Version for compatibility.
    pub version: u32,

    /// Optional description.
    pub description: Option<String>,
}

/// A serializable snapshot of a memory trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot<T, const DIM: usize> {
    /// The trace data.
    pub trace: TropicalDualClifford<T, DIM>,

    /// Number of items.
    pub item_count: u64,

    /// Temperature parameter.
    pub beta: T,

    /// Operation counter.
    pub operation_counter: u64,

    /// Metadata.
    pub metadata: SnapshotMetadata,
}

impl<T: MinuetFloat + Serialize + for<'de> Deserialize<'de>, const DIM: usize> Snapshot<T, DIM> {
    /// Create a snapshot from a memory trace.
    pub fn from_trace<S>(trace: &MemoryTrace<T, DIM, S>) -> Result<Self> {
        Ok(Self {
            trace: trace.raw_trace(),
            item_count: trace.item_count(),
            beta: trace.beta(),
            operation_counter: trace.item_count(), // Approximation
            metadata: SnapshotMetadata {
                created_at: SystemTime::now(),
                item_count: trace.item_count(),
                dimension: DIM,
                version: 1,
                description: None,
            },
        })
    }

    /// Convert back to a memory trace.
    pub fn into_trace(self) -> Result<MemoryTrace<T, DIM>> {
        let snapshot = crate::memory::trace::TraceSnapshot {
            trace: self.trace,
            item_count: self.item_count,
            beta: self.beta,
            operation_counter: self.operation_counter,
        };

        Ok(MemoryTrace::from_snapshot(snapshot))
    }

    /// Save to file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }

    /// Load from file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let snapshot = bincode::deserialize_from(reader)?;
        Ok(snapshot)
    }

    /// Set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.metadata.description = Some(desc.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_roundtrip() {
        let trace: MemoryTrace<f64, 64> = MemoryTrace::new().into_unknown();

        let key = TropicalDualClifford::random();
        let value = TropicalDualClifford::random();
        trace.store(&key, &value).unwrap();

        let snapshot = Snapshot::from_trace(&trace).unwrap();
        let restored = snapshot.into_trace().unwrap();

        assert_eq!(restored.item_count(), 1);
    }
}
