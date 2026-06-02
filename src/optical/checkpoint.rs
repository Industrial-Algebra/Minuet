// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Checkpointed optical memory with journal-based persistence.
//!
//! `CheckpointedOpticalMemory` provides a holographic memory system with:
//! - Fast optical hot paths for store/retrieve
//! - Periodic checkpoint persistence
//! - Hardware-independent recovery
//!
//! # Design Principles
//!
//! - `store()` and `retrieve()` are optical hot paths (minimal overhead)
//! - Persistence happens via periodic checkpoints (not per-operation)
//! - Journal is source of truth; optical state is derived/cached

use std::path::PathBuf;
use std::time::{Duration, Instant};

use amari_holographic::optical::{
    CodebookConfig, GeometricLeeEncoder, LeeEncoderConfig, OpticalCodebook, OpticalFieldAlgebra,
    OpticalRotorField, SymbolId,
};

use super::fingerprint::{FingerprintValidation, TMatrixFingerprint};
use super::hardware::{HardwareCalibration, HardwareError, OpticalHardware};
use super::journal::{
    CompactedMemoryState, JournalError, MemoryJournal, MemoryOp, StoredAssociation,
};
use super::now_timestamp;
use super::symbolic::SymbolicExpression;

/// Configuration for checkpoint behavior.
#[derive(Clone, Debug)]
pub struct CheckpointConfig {
    /// How often to checkpoint (default: 5 minutes).
    pub interval: Duration,
    /// Maximum ops before forcing compaction.
    pub max_ops_before_compact: usize,
    /// Path for journal storage.
    pub journal_path: PathBuf,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_mins(5),
            max_ops_before_compact: 10_000,
            journal_path: PathBuf::from("memory_journal.bin"),
        }
    }
}

impl CheckpointConfig {
    /// Create config with custom journal path.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            journal_path: path.into(),
            ..Default::default()
        }
    }

    /// Set checkpoint interval.
    pub fn interval(mut self, duration: Duration) -> Self {
        self.interval = duration;
        self
    }

    /// Set maximum operations before compaction.
    pub fn max_ops(mut self, max: usize) -> Self {
        self.max_ops_before_compact = max;
        self
    }
}

/// Optical memory with checkpoint-based persistence.
///
/// This is the main entry point for optical holographic memory. It combines:
/// - Hardware abstraction (real or simulated)
/// - Optical field algebra operations
/// - Journal-based persistence
/// - Automatic checkpointing
///
/// # Hot Paths
///
/// `store()` and `retrieve()` are designed as hot paths:
/// - No I/O on store (just buffer ops for later checkpoint)
/// - Zero persistence overhead on retrieve
///
/// # Example
///
/// ```ignore
/// use minuet::optical::*;
///
/// let hardware = MockOpticalHardware::new(42);
/// let mut memory = CheckpointedOpticalMemory::new(
///     hardware,
///     LeeEncoderConfig::default(),
///     CodebookConfig::default(),
///     CheckpointConfig::default(),
/// )?;
///
/// // Store associations
/// memory.store(
///     SymbolicExpression::role_filler("AGENT", "John"),
///     SymbolicExpression::role_filler("ACTION", "run"),
/// )?;
///
/// // Retrieve
/// if let Some(result) = memory.retrieve(&SymbolicExpression::role_filler("AGENT", "John"))? {
///     println!("Found: {:?} (similarity: {:.2})", result.value, result.similarity);
/// }
///
/// // Checkpoint
/// memory.checkpoint()?;
/// ```
pub struct CheckpointedOpticalMemory<H: OpticalHardware> {
    // === Optical Backend ===
    hardware: H,
    algebra: OpticalFieldAlgebra,
    encoder: GeometricLeeEncoder,
    codebook: OpticalCodebook,
    calibration: Option<HardwareCalibration>,

    // === Logical State (derived from journal) ===
    /// Current in-memory state (for fast retrieval).
    logical_state: CompactedMemoryState,

    // === Persistence ===
    journal: MemoryJournal,
    unsaved_ops: Vec<MemoryOp>,

    // === Checkpoint Timing ===
    config: CheckpointConfig,
    last_checkpoint: Instant,
}

impl<H: OpticalHardware> CheckpointedOpticalMemory<H> {
    /// Create new memory system with fresh state.
    pub fn new(
        hardware: H,
        encoder_config: LeeEncoderConfig,
        codebook_config: CodebookConfig,
        checkpoint_config: CheckpointConfig,
    ) -> Result<Self, MemoryError> {
        let algebra = OpticalFieldAlgebra::new(hardware.dimensions());
        let encoder = GeometricLeeEncoder::new(encoder_config.clone());
        let codebook = OpticalCodebook::new(codebook_config.clone());

        let journal = MemoryJournal::new(encoder_config, codebook_config);
        let logical_state = journal.replay_to_state();

        let mut memory = Self {
            hardware,
            algebra,
            encoder,
            codebook,
            calibration: None,
            logical_state,
            journal,
            unsaved_ops: Vec::new(),
            config: checkpoint_config,
            last_checkpoint: Instant::now(),
        };

        // Initial calibration
        memory.calibrate()?;

        Ok(memory)
    }

    /// Restore from checkpoint on (possibly different) hardware.
    pub fn restore(hardware: H, checkpoint_config: CheckpointConfig) -> Result<Self, MemoryError> {
        // Load journal
        let journal =
            MemoryJournal::load(&checkpoint_config.journal_path).map_err(MemoryError::Journal)?;

        // Rebuild codebook from seeds
        let mut codebook = OpticalCodebook::new(journal.codebook_config.clone());
        let logical_state = journal.replay_to_state();
        codebook.import_seeds(logical_state.symbol_seeds.clone());

        // Create encoder
        let encoder = GeometricLeeEncoder::new(journal.encoder_config.clone());
        let algebra = OpticalFieldAlgebra::new(hardware.dimensions());

        let mut memory = Self {
            hardware,
            algebra,
            encoder,
            codebook,
            calibration: None,
            logical_state,
            journal,
            unsaved_ops: Vec::new(),
            config: checkpoint_config,
            last_checkpoint: Instant::now(),
        };

        // Validate/recalibrate hardware
        memory.validate_and_calibrate()?;

        Ok(memory)
    }

    /// Store a key-value association.
    ///
    /// This is a hot path: optical operation + buffer op for checkpoint.
    /// No I/O happens here.
    pub fn store(
        &mut self,
        key: SymbolicExpression,
        value: SymbolicExpression,
    ) -> Result<(), MemoryError> {
        let timestamp = now_timestamp();

        // 1. Ensure symbols are registered
        self.ensure_symbols_registered(&key)?;
        self.ensure_symbols_registered(&value)?;

        // 2. Instantiate to rotor fields
        let key_field = self.instantiate(&key)?;
        let value_field = self.instantiate(&value)?;

        // 3. Optical store (bind key with value, add to memory)
        self.optical_store(&key_field, &value_field)?;

        // 4. Update logical state
        let assoc = StoredAssociation {
            key: key.clone(),
            value: value.clone(),
            strength: 1.0,
            created_at: timestamp,
            last_accessed: timestamp,
        };

        // Update or insert
        if let Some(existing) = self.logical_state.find_by_key_mut(&key) {
            *existing = assoc;
        } else {
            self.logical_state.associations.push(assoc);
        }

        // 5. Buffer op for checkpoint (NO I/O here!)
        self.unsaved_ops.push(MemoryOp::Store {
            key,
            value,
            strength: 1.0,
            timestamp,
        });

        // 6. Maybe checkpoint
        self.maybe_checkpoint()?;

        Ok(())
    }

    /// Retrieve value associated with key.
    ///
    /// This is a hot path: pure optical operation with zero persistence overhead.
    pub fn retrieve(
        &mut self,
        query: &SymbolicExpression,
    ) -> Result<Option<RetrievalResult>, MemoryError> {
        // Instantiate query
        let query_field = self.instantiate(query)?;

        // Collect keys and strengths to avoid borrow conflict
        let assoc_data: Vec<(usize, SymbolicExpression, f32)> = self
            .logical_state
            .associations
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.key.clone(), a.strength))
            .collect();

        // Optical similarity search
        let mut best_match: Option<(usize, f32)> = None;

        for (i, key, strength) in assoc_data {
            let key_field = self.instantiate(&key)?;
            let sim = self.algebra.similarity(&query_field, &key_field);

            let weighted_sim = sim * strength;

            if let Some((_, best_sim)) = best_match {
                if weighted_sim > best_sim {
                    best_match = Some((i, weighted_sim));
                }
            } else if weighted_sim > 0.5 {
                // Threshold
                best_match = Some((i, weighted_sim));
            }
        }

        Ok(best_match.map(|(i, sim)| {
            let assoc = &self.logical_state.associations[i];
            RetrievalResult {
                value: assoc.value.clone(),
                similarity: sim,
                strength: assoc.strength,
            }
        }))
    }

    /// Register a new symbol.
    pub fn register_symbol(&mut self, name: impl Into<String>) -> Result<SymbolId, MemoryError> {
        let symbol = SymbolId::new(name);

        if !self.codebook.contains(&symbol) {
            self.codebook.register(symbol.clone());

            let seed = self.codebook.get_seed(&symbol);
            self.unsaved_ops.push(MemoryOp::RegisterSymbol {
                symbol: symbol.clone(),
                seed,
                timestamp: now_timestamp(),
            });
        }

        Ok(symbol)
    }

    /// Force checkpoint now.
    pub fn checkpoint(&mut self) -> Result<(), MemoryError> {
        // 1. Append buffered ops to journal
        self.journal.ops.append(&mut self.unsaved_ops);

        // 2. Update T-matrix fingerprint
        self.journal.t_fingerprint = Some(
            TMatrixFingerprint::capture(&mut self.hardware, TMatrixFingerprint::DEFAULT_N_PROBES)
                .map_err(MemoryError::Hardware)?,
        );

        // 3. Save journal
        self.journal
            .save(&self.config.journal_path)
            .map_err(MemoryError::Journal)?;

        // 4. Compact if needed
        if self.journal.ops.len() > self.config.max_ops_before_compact {
            self.journal.compact();
            self.journal
                .save(&self.config.journal_path)
                .map_err(MemoryError::Journal)?;
        }

        self.last_checkpoint = Instant::now();
        Ok(())
    }

    /// Apply global decay to all memories.
    pub fn decay(&mut self, factor: f32) -> Result<(), MemoryError> {
        for assoc in &mut self.logical_state.associations {
            assoc.strength *= factor;
        }
        self.logical_state
            .associations
            .retain(|a| a.strength > 0.01);

        self.unsaved_ops.push(MemoryOp::Decay {
            factor,
            timestamp: now_timestamp(),
        });

        self.maybe_checkpoint()
    }

    /// Forget a specific association.
    pub fn forget(&mut self, key: &SymbolicExpression) -> Result<(), MemoryError> {
        self.logical_state.associations.retain(|a| &a.key != key);

        self.unsaved_ops.push(MemoryOp::Forget {
            key: key.clone(),
            timestamp: now_timestamp(),
        });

        self.maybe_checkpoint()
    }

    /// Strengthen an existing association.
    pub fn strengthen(&mut self, key: &SymbolicExpression, delta: f32) -> Result<(), MemoryError> {
        if let Some(assoc) = self.logical_state.find_by_key_mut(key) {
            assoc.strength += delta;
            assoc.last_accessed = now_timestamp();

            self.unsaved_ops.push(MemoryOp::Strengthen {
                key: key.clone(),
                delta,
                timestamp: now_timestamp(),
            });
        }

        self.maybe_checkpoint()
    }

    /// Get current hardware info.
    pub fn hardware_info(&self) -> HardwareInfo {
        HardwareInfo {
            id: self.hardware.id().to_string(),
            dimensions: self.hardware.dimensions(),
            n_modes: self.hardware.n_modes(),
            is_ready: self.hardware.is_ready(),
            is_calibrated: self.calibration.is_some(),
        }
    }

    /// Get memory statistics.
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            n_associations: self.logical_state.associations.len(),
            n_symbols: self.logical_state.symbol_seeds.len(),
            n_unsaved_ops: self.unsaved_ops.len(),
            journal_ops: self.journal.ops.len(),
            has_base_state: self.journal.base_state.is_some(),
        }
    }

    /// Get all current associations.
    pub fn associations(&self) -> &[StoredAssociation] {
        &self.logical_state.associations
    }

    /// Get mutable access to hardware (for advanced use).
    pub fn hardware_mut(&mut self) -> &mut H {
        &mut self.hardware
    }

    /// Get the encoder.
    pub fn encoder(&self) -> &GeometricLeeEncoder {
        &self.encoder
    }

    /// Get the codebook.
    pub fn codebook(&self) -> &OpticalCodebook {
        &self.codebook
    }

    // === Private Implementation ===

    fn maybe_checkpoint(&mut self) -> Result<(), MemoryError> {
        if self.last_checkpoint.elapsed() >= self.config.interval {
            self.checkpoint()?;
        }
        Ok(())
    }

    fn calibrate(&mut self) -> Result<(), MemoryError> {
        let cal = self
            .hardware
            .full_calibrate()
            .map_err(MemoryError::Hardware)?;
        self.calibration = Some(cal);
        Ok(())
    }

    fn validate_and_calibrate(&mut self) -> Result<(), MemoryError> {
        // Check fingerprint if available
        let validation = if let Some(ref fp) = self.journal.t_fingerprint {
            fp.validate(&mut self.hardware)
                .map_err(MemoryError::Hardware)?
        } else {
            FingerprintValidation::NoFingerprint
        };

        match validation {
            FingerprintValidation::Valid => {
                // Quick calibration sufficient
                let cal = self
                    .hardware
                    .quick_calibrate()
                    .map_err(MemoryError::Hardware)?;
                self.calibration = Some(cal);
            }
            _ => {
                // Full recalibration needed
                self.calibrate()?;
            }
        }

        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn ensure_symbols_registered(&mut self, expr: &SymbolicExpression) -> Result<(), MemoryError> {
        for symbol in expr.referenced_symbols() {
            if !self.codebook.contains(symbol) {
                self.codebook.register(symbol.clone());

                let seed = self.codebook.get_seed(symbol);
                self.unsaved_ops.push(MemoryOp::RegisterSymbol {
                    symbol: symbol.clone(),
                    seed,
                    timestamp: now_timestamp(),
                });
            }
        }
        Ok(())
    }

    fn instantiate(&mut self, expr: &SymbolicExpression) -> Result<OpticalRotorField, MemoryError> {
        match expr {
            SymbolicExpression::Symbol(id) => self
                .codebook
                .get(id)
                .cloned()
                .ok_or_else(|| MemoryError::UnknownSymbol(id.clone())),

            SymbolicExpression::Bind(a, b) => {
                let field_a = self.instantiate(a)?;
                let field_b = self.instantiate(b)?;
                Ok(self.algebra.bind(&field_a, &field_b))
            }

            SymbolicExpression::Bundle(elements) => {
                let fields: Vec<OpticalRotorField> = elements
                    .iter()
                    .map(|(_, e)| self.instantiate(e))
                    .collect::<Result<_, _>>()?;
                let weights: Vec<f32> = elements.iter().map(|(w, _)| w.0).collect();
                Ok(self.algebra.bundle(&fields, &weights))
            }
        }
    }

    #[allow(clippy::unused_self)]
    #[allow(clippy::unnecessary_wraps)]
    fn optical_store(
        &mut self,
        _key: &OpticalRotorField,
        _value: &OpticalRotorField,
    ) -> Result<(), MemoryError> {
        // In a full implementation, this would:
        // 1. Bind key with value to create memory trace
        // 2. Bundle with existing memory (superposition)
        // 3. Optionally display and measure for resonator cleanup
        //
        // For now, we rely on the logical state for retrieval
        Ok(())
    }
}

/// Result of a retrieval query.
#[derive(Clone, Debug)]
pub struct RetrievalResult {
    /// Retrieved value expression.
    pub value: SymbolicExpression,
    /// Similarity score (weighted by strength).
    pub similarity: f32,
    /// Association strength.
    pub strength: f32,
}

/// Hardware information.
#[derive(Clone, Debug)]
pub struct HardwareInfo {
    /// Hardware identifier.
    pub id: String,
    /// Grid dimensions.
    pub dimensions: (usize, usize),
    /// Number of optical modes.
    pub n_modes: usize,
    /// Whether hardware is ready.
    pub is_ready: bool,
    /// Whether hardware is calibrated.
    pub is_calibrated: bool,
}

/// Memory statistics.
#[derive(Clone, Debug)]
pub struct MemoryStats {
    /// Number of stored associations.
    pub n_associations: usize,
    /// Number of registered symbols.
    pub n_symbols: usize,
    /// Number of unsaved operations.
    pub n_unsaved_ops: usize,
    /// Number of operations in journal.
    pub journal_ops: usize,
    /// Whether journal has a base state.
    pub has_base_state: bool,
}

/// Memory system errors.
#[derive(Debug)]
pub enum MemoryError {
    /// Hardware error.
    Hardware(HardwareError),
    /// Journal error.
    Journal(JournalError),
    /// Unknown symbol.
    UnknownSymbol(SymbolId),
    /// Dimension mismatch.
    DimensionMismatch,
    /// Not calibrated.
    NotCalibrated,
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hardware(e) => write!(f, "hardware error: {e}"),
            Self::Journal(e) => write!(f, "journal error: {e}"),
            Self::UnknownSymbol(id) => write!(f, "unknown symbol: {id}"),
            Self::DimensionMismatch => write!(f, "dimension mismatch"),
            Self::NotCalibrated => write!(f, "hardware not calibrated"),
        }
    }
}

impl std::error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Hardware(e) => Some(e),
            Self::Journal(e) => Some(e),
            _ => None,
        }
    }
}

impl From<HardwareError> for MemoryError {
    fn from(e: HardwareError) -> Self {
        Self::Hardware(e)
    }
}

impl From<JournalError> for MemoryError {
    fn from(e: JournalError) -> Self {
        Self::Journal(e)
    }
}
