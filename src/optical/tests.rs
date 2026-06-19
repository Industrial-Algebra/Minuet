// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for the optical module.

use super::*;
use amari_holographic::optical::{CodebookConfig, LeeEncoderConfig};
use std::time::Duration;
use tempfile::tempdir;

fn test_encoder_config() -> LeeEncoderConfig {
    LeeEncoderConfig {
        carrier_frequency: 0.25,
        carrier_angle: 0.0,
        dimensions: (256, 256),
    }
}

fn test_codebook_config() -> CodebookConfig {
    CodebookConfig {
        dimensions: (256, 256),
        base_seed: 12345,
    }
}

#[test]
fn test_symbolic_expression_roundtrip() {
    let expr = SymbolicExpression::bind(
        SymbolicExpression::symbol("AGENT"),
        SymbolicExpression::symbol("John"),
    );

    let json = serde_json::to_string(&expr).unwrap();
    let restored: SymbolicExpression = serde_json::from_str(&json).unwrap();

    assert_eq!(expr, restored);
}

#[test]
fn test_journal_replay_consistency() {
    let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

    // Record operations
    journal.ops.push(MemoryOp::RegisterSymbol {
        symbol: amari_holographic::optical::SymbolId::new("AGENT"),
        seed: Some(12345),
        timestamp: 1000,
    });
    journal.ops.push(MemoryOp::Store {
        key: SymbolicExpression::symbol("test_key"),
        value: SymbolicExpression::symbol("test_value"),
        strength: 1.0,
        timestamp: 2000,
    });

    let state = journal.replay_to_state();

    assert_eq!(state.associations.len(), 1);
    assert!(state
        .symbol_seeds
        .contains_key(&amari_holographic::optical::SymbolId::new("AGENT")));
}

#[test]
fn test_checkpoint_restore_same_hardware() {
    let dir = tempdir().unwrap();
    let journal_path = dir.path().join("test_journal.bin");

    let hardware = MockOpticalHardware::new(42);
    let config = CheckpointConfig {
        journal_path: journal_path.clone(),
        interval: Duration::from_secs(3600), // Long interval to control checkpointing
        ..Default::default()
    };

    let mut memory = CheckpointedOpticalMemory::new(
        hardware,
        test_encoder_config(),
        test_codebook_config(),
        config.clone(),
    )
    .unwrap();

    // Store some memories
    memory
        .store(
            SymbolicExpression::role_filler("AGENT", "John"),
            SymbolicExpression::role_filler("ACTION", "run"),
        )
        .unwrap();

    memory.checkpoint().unwrap();

    // Simulate restart with same hardware
    drop(memory);
    let same_hardware = MockOpticalHardware::new(42);
    let mut restored = CheckpointedOpticalMemory::restore(same_hardware, config).unwrap();

    // Should retrieve
    let result = restored
        .retrieve(&SymbolicExpression::role_filler("AGENT", "John"))
        .unwrap();

    assert!(result.is_some());
}

#[test]
fn test_checkpoint_restore_different_hardware() {
    let dir = tempdir().unwrap();
    let journal_path = dir.path().join("test_journal.bin");

    let hardware = MockOpticalHardware::new(42);
    let config = CheckpointConfig {
        journal_path: journal_path.clone(),
        interval: Duration::from_secs(3600),
        ..Default::default()
    };

    let mut memory = CheckpointedOpticalMemory::new(
        hardware,
        test_encoder_config(),
        test_codebook_config(),
        config.clone(),
    )
    .unwrap();

    memory
        .store(
            SymbolicExpression::role_filler("AGENT", "John"),
            SymbolicExpression::role_filler("ACTION", "run"),
        )
        .unwrap();

    memory.checkpoint().unwrap();

    // Restart with DIFFERENT hardware
    drop(memory);
    let different_hardware = MockOpticalHardware::new(999); // Different seed = different T
    let mut restored = CheckpointedOpticalMemory::restore(different_hardware, config).unwrap();

    // Should still retrieve (recalibrated to new hardware)
    let result = restored
        .retrieve(&SymbolicExpression::role_filler("AGENT", "John"))
        .unwrap();

    assert!(result.is_some());
}

#[test]
fn test_fingerprint_detects_drift() {
    let mut hardware = MockOpticalHardware::new(42);
    let fingerprint = TMatrixFingerprint::capture(&mut hardware, 5).unwrap();

    // Should be valid immediately
    let validation = fingerprint.validate(&mut hardware).unwrap();
    assert!(matches!(validation, FingerprintValidation::Valid));

    // Simulate significant drift
    hardware.drift_t_matrix(0.5);

    // Should detect drift or different hardware
    let validation = fingerprint.validate(&mut hardware).unwrap();
    assert!(!matches!(validation, FingerprintValidation::Valid));
}

#[test]
fn test_fingerprint_detects_different_hardware() {
    let mut hardware1 = MockOpticalHardware::new(42);
    let fingerprint = TMatrixFingerprint::capture(&mut hardware1, 5).unwrap();

    // Validate against different hardware
    let mut hardware2 = MockOpticalHardware::new(999);
    let validation = fingerprint.validate(&mut hardware2).unwrap();

    assert!(matches!(
        validation,
        FingerprintValidation::DifferentHardware { .. }
    ));
}

#[test]
fn test_store_retrieve_basic() {
    let dir = tempdir().unwrap();
    let journal_path = dir.path().join("test_journal.bin");

    let hardware = MockOpticalHardware::new(42);
    let config = CheckpointConfig {
        journal_path,
        interval: Duration::from_secs(3600),
        ..Default::default()
    };

    let mut memory = CheckpointedOpticalMemory::new(
        hardware,
        test_encoder_config(),
        test_codebook_config(),
        config,
    )
    .unwrap();

    // Store
    memory
        .store(
            SymbolicExpression::symbol("cat"),
            SymbolicExpression::symbol("meow"),
        )
        .unwrap();

    memory
        .store(
            SymbolicExpression::symbol("dog"),
            SymbolicExpression::symbol("bark"),
        )
        .unwrap();

    // Retrieve
    let cat_result = memory.retrieve(&SymbolicExpression::symbol("cat")).unwrap();
    assert!(cat_result.is_some());
    assert_eq!(
        cat_result.unwrap().value,
        SymbolicExpression::symbol("meow")
    );

    let dog_result = memory.retrieve(&SymbolicExpression::symbol("dog")).unwrap();
    assert!(dog_result.is_some());
    assert_eq!(
        dog_result.unwrap().value,
        SymbolicExpression::symbol("bark")
    );
}

#[test]
fn test_memory_decay() {
    let dir = tempdir().unwrap();
    let journal_path = dir.path().join("test_journal.bin");

    let hardware = MockOpticalHardware::new(42);
    let config = CheckpointConfig {
        journal_path,
        interval: Duration::from_secs(3600),
        ..Default::default()
    };

    let mut memory = CheckpointedOpticalMemory::new(
        hardware,
        test_encoder_config(),
        test_codebook_config(),
        config,
    )
    .unwrap();

    memory
        .store(
            SymbolicExpression::symbol("key"),
            SymbolicExpression::symbol("value"),
        )
        .unwrap();

    // Apply decay
    memory.decay(0.5).unwrap();

    let stats = memory.stats();
    assert_eq!(stats.n_associations, 1);

    // Apply more decay until association is removed
    memory.decay(0.01).unwrap();

    let stats = memory.stats();
    assert_eq!(stats.n_associations, 0);
}

#[test]
fn test_memory_forget() {
    let dir = tempdir().unwrap();
    let journal_path = dir.path().join("test_journal.bin");

    let hardware = MockOpticalHardware::new(42);
    let config = CheckpointConfig {
        journal_path,
        interval: Duration::from_secs(3600),
        ..Default::default()
    };

    let mut memory = CheckpointedOpticalMemory::new(
        hardware,
        test_encoder_config(),
        test_codebook_config(),
        config,
    )
    .unwrap();

    memory
        .store(
            SymbolicExpression::symbol("key1"),
            SymbolicExpression::symbol("value1"),
        )
        .unwrap();
    memory
        .store(
            SymbolicExpression::symbol("key2"),
            SymbolicExpression::symbol("value2"),
        )
        .unwrap();

    assert_eq!(memory.stats().n_associations, 2);

    memory.forget(&SymbolicExpression::symbol("key1")).unwrap();

    assert_eq!(memory.stats().n_associations, 1);

    let result = memory
        .retrieve(&SymbolicExpression::symbol("key1"))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_hardware_info() {
    let dir = tempdir().unwrap();
    let journal_path = dir.path().join("test_journal.bin");

    let hardware = MockOpticalHardware::new(42);
    let config = CheckpointConfig {
        journal_path,
        ..Default::default()
    };

    let memory = CheckpointedOpticalMemory::new(
        hardware,
        test_encoder_config(),
        test_codebook_config(),
        config,
    )
    .unwrap();

    let info = memory.hardware_info();
    assert!(info.is_ready);
    assert!(info.is_calibrated);
    assert_eq!(info.dimensions, (256, 256));
    assert_eq!(info.n_modes, 100);
}

#[test]
fn test_journal_compaction() {
    let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());

    // Add many operations
    for i in 0..100 {
        journal.append(MemoryOp::store(
            SymbolicExpression::symbol(format!("key{}", i)),
            SymbolicExpression::symbol(format!("value{}", i)),
            1.0,
        ));
    }

    assert_eq!(journal.ops.len(), 100);
    assert!(journal.base_state.is_none());

    // Compact
    journal.compact();

    assert!(journal.ops.is_empty());
    assert!(journal.base_state.is_some());
    assert_eq!(journal.base_state.as_ref().unwrap().associations.len(), 100);
}

#[test]
fn test_journal_save_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test_journal.bin");

    let mut journal = MemoryJournal::new(test_encoder_config(), test_codebook_config());
    journal.append(MemoryOp::store(
        SymbolicExpression::symbol("key"),
        SymbolicExpression::symbol("value"),
        1.0,
    ));

    // Save
    journal.save(&path).unwrap();

    // Load
    let loaded = MemoryJournal::load(&path).unwrap();
    assert_eq!(loaded.ops.len(), 1);
}
