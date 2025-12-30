//! Memory operation journal for checkpoint persistence.
//!
//! The journal provides append-only logging of memory operations, enabling:
//! - Replay to reconstruct logical state
//! - Compaction to reduce storage overhead
//! - Portable persistence across hardware configurations

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use amari_holographic::optical::{CodebookConfig, LeeEncoderConfig, SymbolId};
use serde::{Deserialize, Serialize};

use super::fingerprint::TMatrixFingerprint;
use super::now_timestamp;
use super::symbolic::SymbolicExpression;

/// A single memory operation (lightweight, serializable).
///
/// Operations are designed to be small and fast to serialize,
/// enabling efficient append-only journaling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryOp {
    /// Store a key-value association.
    Store {
        /// The key expression.
        key: SymbolicExpression,
        /// The value expression.
        value: SymbolicExpression,
        /// Association strength (typically 1.0 for new associations).
        strength: f32,
        /// Operation timestamp (millis since epoch).
        timestamp: u64,
    },

    /// Strengthen an existing association.
    Strengthen {
        /// The key to strengthen.
        key: SymbolicExpression,
        /// Amount to add to strength.
        delta: f32,
        /// Operation timestamp.
        timestamp: u64,
    },

    /// Apply global decay to all memories.
    Decay {
        /// Multiply all strengths by this factor (0.0 to 1.0).
        factor: f32,
        /// Operation timestamp.
        timestamp: u64,
    },

    /// Remove a specific association.
    Forget {
        /// The key to forget.
        key: SymbolicExpression,
        /// Operation timestamp.
        timestamp: u64,
    },

    /// Register a new symbol in the codebook.
    RegisterSymbol {
        /// The symbol to register.
        symbol: SymbolId,
        /// Seed for deterministic field generation (None = auto-generate).
        seed: Option<u64>,
        /// Operation timestamp.
        timestamp: u64,
    },
}

impl MemoryOp {
    /// Get the timestamp of this operation.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Store { timestamp, .. }
            | Self::Strengthen { timestamp, .. }
            | Self::Decay { timestamp, .. }
            | Self::Forget { timestamp, .. }
            | Self::RegisterSymbol { timestamp, .. } => *timestamp,
        }
    }

    /// Create a store operation with current timestamp.
    pub fn store(key: SymbolicExpression, value: SymbolicExpression, strength: f32) -> Self {
        Self::Store {
            key,
            value,
            strength,
            timestamp: now_timestamp(),
        }
    }

    /// Create a strengthen operation with current timestamp.
    pub fn strengthen(key: SymbolicExpression, delta: f32) -> Self {
        Self::Strengthen {
            key,
            delta,
            timestamp: now_timestamp(),
        }
    }

    /// Create a decay operation with current timestamp.
    pub fn decay(factor: f32) -> Self {
        Self::Decay {
            factor,
            timestamp: now_timestamp(),
        }
    }

    /// Create a forget operation with current timestamp.
    pub fn forget(key: SymbolicExpression) -> Self {
        Self::Forget {
            key,
            timestamp: now_timestamp(),
        }
    }

    /// Create a register symbol operation with current timestamp.
    pub fn register_symbol(symbol: SymbolId, seed: Option<u64>) -> Self {
        Self::RegisterSymbol {
            symbol,
            seed,
            timestamp: now_timestamp(),
        }
    }
}

/// A stored key-value association with metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredAssociation {
    /// The key expression.
    pub key: SymbolicExpression,
    /// The value expression.
    pub value: SymbolicExpression,
    /// Current strength (decays over time).
    pub strength: f32,
    /// When this association was first created.
    pub created_at: u64,
    /// When this association was last accessed.
    pub last_accessed: u64,
}

/// Compacted snapshot of logical memory state.
///
/// This represents the full state of memory at a point in time,
/// suitable for checkpointing and fast restore.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompactedMemoryState {
    /// All current associations.
    pub associations: Vec<StoredAssociation>,
    /// Symbol registry (symbol → seed for deterministic regeneration).
    pub symbol_seeds: BTreeMap<SymbolId, u64>,
    /// Timestamp when this state was compacted.
    pub compacted_at: u64,
}

impl CompactedMemoryState {
    /// Create an empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Find an association by key.
    pub fn find_by_key(&self, key: &SymbolicExpression) -> Option<&StoredAssociation> {
        self.associations.iter().find(|a| &a.key == key)
    }

    /// Find a mutable association by key.
    pub fn find_by_key_mut(&mut self, key: &SymbolicExpression) -> Option<&mut StoredAssociation> {
        self.associations.iter_mut().find(|a| &a.key == key)
    }
}

/// Append-only journal of memory operations.
///
/// The journal is the source of truth for memory state. It consists of:
/// - An optional base state (compacted history)
/// - A list of operations since the base state
/// - Configuration for encoding and codebook
/// - Optional T-matrix fingerprint for hardware validation
///
/// # Persistence Model
///
/// ```text
/// Journal File
/// ├── base_state: Option<CompactedMemoryState>  ← Compacted history
/// ├── ops: Vec<MemoryOp>                        ← Recent operations
/// ├── encoder_config: LeeEncoderConfig          ← For field generation
/// ├── codebook_config: CodebookConfig           ← For symbol generation
/// ├── t_fingerprint: Option<TMatrixFingerprint> ← For hardware validation
/// └── version: u32                              ← Format version
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryJournal {
    /// Base snapshot (compacted history), if any.
    pub base_state: Option<CompactedMemoryState>,

    /// Operations since base_state (or since beginning if no base).
    pub ops: Vec<MemoryOp>,

    /// Encoder configuration (needed for field generation).
    pub encoder_config: LeeEncoderConfig,

    /// Codebook configuration.
    pub codebook_config: CodebookConfig,

    /// Last known T-matrix fingerprint (for cache validation).
    pub t_fingerprint: Option<TMatrixFingerprint>,

    /// Journal format version (for forward compatibility).
    pub version: u32,
}

impl MemoryJournal {
    /// Current journal format version.
    pub const CURRENT_VERSION: u32 = 1;

    /// Minimum strength threshold below which associations are removed.
    pub const MIN_STRENGTH_THRESHOLD: f32 = 0.01;

    /// Create a new empty journal.
    pub fn new(encoder_config: LeeEncoderConfig, codebook_config: CodebookConfig) -> Self {
        Self {
            base_state: None,
            ops: Vec::new(),
            encoder_config,
            codebook_config,
            t_fingerprint: None,
            version: Self::CURRENT_VERSION,
        }
    }

    /// Replay all operations to compute current logical state.
    ///
    /// This reconstructs the full memory state by:
    /// 1. Starting from base_state (or empty state if none)
    /// 2. Applying each operation in order
    pub fn replay_to_state(&self) -> CompactedMemoryState {
        let mut state = self.base_state.clone().unwrap_or_default();

        for op in &self.ops {
            self.apply_op(&mut state, op);
        }

        state.compacted_at = now_timestamp();
        state
    }

    /// Apply a single operation to a state.
    fn apply_op(&self, state: &mut CompactedMemoryState, op: &MemoryOp) {
        match op {
            MemoryOp::Store {
                key,
                value,
                strength,
                timestamp,
            } => {
                // Check if key already exists
                if let Some(assoc) = state.find_by_key_mut(key) {
                    assoc.value = value.clone();
                    assoc.strength = *strength;
                    assoc.last_accessed = *timestamp;
                } else {
                    state.associations.push(StoredAssociation {
                        key: key.clone(),
                        value: value.clone(),
                        strength: *strength,
                        created_at: *timestamp,
                        last_accessed: *timestamp,
                    });
                }
            }

            MemoryOp::Strengthen {
                key,
                delta,
                timestamp,
            } => {
                if let Some(assoc) = state.find_by_key_mut(key) {
                    assoc.strength += delta;
                    assoc.last_accessed = *timestamp;
                }
            }

            MemoryOp::Decay { factor, .. } => {
                for assoc in &mut state.associations {
                    assoc.strength *= factor;
                }
                // Remove very weak associations
                state
                    .associations
                    .retain(|a| a.strength > Self::MIN_STRENGTH_THRESHOLD);
            }

            MemoryOp::Forget { key, .. } => {
                state.associations.retain(|a| &a.key != key);
            }

            MemoryOp::RegisterSymbol { symbol, seed, .. } => {
                let actual_seed = seed.unwrap_or_else(|| self.generate_symbol_seed(symbol));
                state.symbol_seeds.insert(symbol.clone(), actual_seed);
            }
        }
    }

    /// Generate a deterministic seed for a symbol.
    fn generate_symbol_seed(&self, symbol: &SymbolId) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.codebook_config.base_seed.hash(&mut hasher);
        symbol.as_str().hash(&mut hasher);
        hasher.finish()
    }

    /// Compact the journal by folding ops into base_state.
    ///
    /// This reduces storage overhead by converting the operation log
    /// into a single snapshot. The result is semantically equivalent.
    pub fn compact(&mut self) {
        let state = self.replay_to_state();
        self.base_state = Some(state);
        self.ops.clear();
    }

    /// Check if the journal should be compacted.
    pub fn should_compact(&self, max_ops: usize) -> bool {
        self.ops.len() > max_ops
    }

    /// Append an operation to the journal.
    pub fn append(&mut self, op: MemoryOp) {
        self.ops.push(op);
    }

    /// Append multiple operations to the journal.
    pub fn append_all(&mut self, ops: impl IntoIterator<Item = MemoryOp>) {
        self.ops.extend(ops);
    }

    /// Total number of operations (base + pending).
    pub fn total_ops(&self) -> usize {
        let base_ops = self.base_state.as_ref().map_or(0, |s| s.associations.len());
        base_ops + self.ops.len()
    }

    /// Save journal to file.
    pub fn save(&self, path: &Path) -> Result<(), JournalError> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }

    /// Load journal from file.
    pub fn load(path: &Path) -> Result<Self, JournalError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let journal: Self = bincode::deserialize_from(reader)?;

        if journal.version > Self::CURRENT_VERSION {
            return Err(JournalError::UnsupportedVersion(journal.version));
        }

        Ok(journal)
    }
}

/// Errors that can occur during journal operations.
#[derive(Debug)]
pub enum JournalError {
    /// I/O error during file operations.
    Io(std::io::Error),
    /// Serialization/deserialization error.
    Serialization(Box<bincode::ErrorKind>),
    /// Journal version is newer than supported.
    UnsupportedVersion(u32),
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "journal I/O error: {e}"),
            Self::Serialization(e) => write!(f, "journal serialization error: {e}"),
            Self::UnsupportedVersion(v) => {
                let current = MemoryJournal::CURRENT_VERSION;
                write!(
                    f,
                    "journal version {v} is newer than supported version {current}"
                )
            }
        }
    }
}

impl std::error::Error for JournalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Serialization(e) => Some(e),
            Self::UnsupportedVersion(_) => None,
        }
    }
}

impl From<std::io::Error> for JournalError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<Box<bincode::ErrorKind>> for JournalError {
    fn from(e: Box<bincode::ErrorKind>) -> Self {
        Self::Serialization(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encoder_config() -> LeeEncoderConfig {
        LeeEncoderConfig {
            carrier_frequency: 0.25,
            carrier_angle: 0.0,
            dimensions: (64, 64),
        }
    }

    fn test_codebook_config() -> CodebookConfig {
        CodebookConfig {
            dimensions: (64, 64),
            base_seed: 12345,
        }
    }

    #[test]
    fn test_journal_new() {
        let journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());
        assert!(journal.base_state.is_none());
        assert!(journal.ops.is_empty());
        assert_eq!(journal.version, MemoryJournal::CURRENT_VERSION);
    }

    #[test]
    fn test_journal_replay_store() {
        let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

        journal.append(MemoryOp::store(
            SymbolicExpression::symbol("key1"),
            SymbolicExpression::symbol("value1"),
            1.0,
        ));

        let state = journal.replay_to_state();
        assert_eq!(state.associations.len(), 1);
        assert_eq!(
            state.associations[0].key,
            SymbolicExpression::symbol("key1")
        );
    }

    #[test]
    fn test_journal_replay_decay() {
        let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

        journal.append(MemoryOp::store(
            SymbolicExpression::symbol("key1"),
            SymbolicExpression::symbol("value1"),
            1.0,
        ));
        journal.append(MemoryOp::decay(0.5));

        let state = journal.replay_to_state();
        assert_eq!(state.associations.len(), 1);
        assert!((state.associations[0].strength - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_journal_replay_forget() {
        let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

        journal.append(MemoryOp::store(
            SymbolicExpression::symbol("key1"),
            SymbolicExpression::symbol("value1"),
            1.0,
        ));
        journal.append(MemoryOp::forget(SymbolicExpression::symbol("key1")));

        let state = journal.replay_to_state();
        assert!(state.associations.is_empty());
    }

    #[test]
    fn test_journal_compact() {
        let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

        for i in 0..10 {
            journal.append(MemoryOp::store(
                SymbolicExpression::symbol(format!("key{}", i)),
                SymbolicExpression::symbol(format!("value{}", i)),
                1.0,
            ));
        }

        assert_eq!(journal.ops.len(), 10);

        journal.compact();

        assert!(journal.ops.is_empty());
        assert!(journal.base_state.is_some());
        assert_eq!(journal.base_state.as_ref().unwrap().associations.len(), 10);
    }

    #[test]
    fn test_journal_register_symbol() {
        let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

        journal.append(MemoryOp::register_symbol(SymbolId::new("AGENT"), Some(42)));

        let state = journal.replay_to_state();
        assert_eq!(state.symbol_seeds.get(&SymbolId::new("AGENT")), Some(&42));
    }
}
