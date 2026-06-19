// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Optical backend for holographic memory with checkpoint-based persistence.
//!
//! This module provides an optical computing backend built on `amari-holographic`'s
//! optical primitives. Key features:
//!
//! - **Hardware abstraction**: `OpticalHardware` trait for DMD/MMF systems
//! - **Checkpoint persistence**: Journal-based state persistence that's portable across hardware
//! - **T-matrix fingerprinting**: Fast hardware state validation without full calibration
//! - **Symbolic expressions**: Hardware-independent memory content representation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        Application Layer                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌───────────────────────────────────────────────────────────────┐ │
//! │  │               CheckpointedOpticalMemory                       │ │
//! │  │  • store() / retrieve() - optical hot paths                  │ │
//! │  │  • checkpoint() - periodic persistence                       │ │
//! │  │  • restore() - hardware-independent recovery                 │ │
//! │  └───────────────────────────────────────────────────────────────┘ │
//! │                              │                                      │
//! │          ┌───────────────────┼───────────────────┐                 │
//! │          ▼                   ▼                   ▼                 │
//! │  ┌─────────────┐   ┌─────────────────┐   ┌─────────────────┐      │
//! │  │MemoryJournal│   │TMatrixFingerprint│   │OpticalHardware │      │
//! │  │  (portable) │   │  (validation)   │   │   (backend)    │      │
//! │  └─────────────┘   └─────────────────┘   └─────────────────┘      │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Persistence Model
//!
//! The optical system is a **compute accelerator**, not a storage medium.
//! Minuet persists the logical state and regenerates hardware-specific state on restore:
//!
//! | Layer | Content | Portability |
//! |-------|---------|-------------|
//! | Semantic | Associations, relationships | Fully portable |
//! | Codebook | Symbol → seed mappings | Regenerable |
//! | Holograms | Binary patterns | Derived on demand |
//! | Calibration | T-matrix, learned patterns | Hardware-bound |
//!
//! # Example
//!
//! ```ignore
//! use minuet::optical::*;
//!
//! // Create memory with mock hardware
//! let hardware = MockOpticalHardware::new(42);
//! let mut memory = CheckpointedOpticalMemory::new(
//!     hardware,
//!     LeeEncoderConfig::default(),
//!     CodebookConfig::default(),
//!     CheckpointConfig::default(),
//! )?;
//!
//! // Store associations
//! memory.store(
//!     SymbolicExpression::role_filler("AGENT", "John"),
//!     SymbolicExpression::role_filler("ACTION", "run"),
//! )?;
//!
//! // Checkpoint (saves to journal)
//! memory.checkpoint()?;
//!
//! // Later, restore on same or different hardware
//! let new_hardware = MockOpticalHardware::new(42);
//! let restored = CheckpointedOpticalMemory::restore(new_hardware, config)?;
//! ```

mod checkpoint;
mod fingerprint;
mod hardware;
mod journal;
mod mock_hardware;
mod symbolic;

pub use checkpoint::{
    CheckpointConfig, CheckpointedOpticalMemory, HardwareInfo, MemoryError, MemoryStats,
    RetrievalResult,
};
pub use fingerprint::{FingerprintValidation, ProbePattern, ProbeResponse, TMatrixFingerprint};
pub use hardware::{HardwareCalibration, HardwareError, OpticalHardware, OpticalMeasurement};
pub use journal::{CompactedMemoryState, JournalError, MemoryJournal, MemoryOp, StoredAssociation};
pub use mock_hardware::MockOpticalHardware;
pub use symbolic::{OrderedFloat, SymbolicExpression};

/// Current time as u64 milliseconds since Unix epoch.
///
/// # Panics
///
/// Panics if the system time is before the Unix epoch (January 1, 1970).
pub fn now_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests;
