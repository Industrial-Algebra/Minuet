// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0
//! Stress tests — production-readiness coverage beyond the unit/integration suite.
//!
//! Four categories surfaced by the PULSE_2026-06-28 gap analysis:
//! 1. Capacity overflow — `RejectPolicy` engages at threshold via the `Pipeline`.
//! 2. Concurrency — many-thread concurrent read/write against `ShardedStore`
//!    (which uses `parking_lot::RwLock` per shard; `MemoryStore: Send + Sync`).
//! 3. Persistence recovery — journal save → drop → reload → replay restores state.
//! 4. Temperature sweep — annealed schedules produce monotonic beta sequences under
//!    varying store load (the contract a noisy backend's cleanup relies on).

use std::sync::Arc;

use amari_holographic::optical::{CodebookConfig, LeeEncoderConfig};
use amari_holographic::ProductCliffordAlgebra;
use minuet::capacity::RejectPolicy;
use minuet::error::MinuetError;
use minuet::optical::MockOpticalHardware;
use minuet::optical::{CheckpointConfig, MemoryJournal, MemoryOp};
use minuet::pipeline::PipelineBuilder;
use minuet::retrieval::{Temperature, TemperatureSchedule};
use minuet::store::ShardedStore;
use minuet::store::SimpleStore;
use minuet::traits::MemoryStore;

/// Small, fast algebra for stress work (64 dims). Large enough that random
/// versors are near-orthogonal (low cross-talk) but small enough that the
/// hundred-item / hundred-thread tests run in well under a second.
type Algebra = ProductCliffordAlgebra<8>;

// ---------------------------------------------------------------------------
// 1. Capacity overflow
// ---------------------------------------------------------------------------

#[test]
fn capacity_overflow_rejects_at_threshold() {
    // A near-saturated pipeline: store until RejectPolicy flips, then assert the
    // next store returns CapacityExceeded (not a silent accept). RejectPolicy is
    // the only place capacity actually gates — SimpleStore always accepts and
    // just attaches a warning; Pipeline::store consults the policy.
    let pipeline = PipelineBuilder::<Algebra>::new()
        .with_capacity_policy(RejectPolicy::with_threshold(0.01))
        .build()
        .expect("pipeline builds");

    // The first store pushes utilization > 0.01 (one item in a ~46-capacity
    // 64-dim trace is already ~2% utilized), so subsequent stores must reject.
    let k = Algebra::random_versor(2);
    let v = Algebra::random_versor(2);
    let first = pipeline.store(&k, &v);
    assert!(first.is_ok(), "first store should succeed: {:?}", first);

    // Keep storing distinct pairs until rejection. With threshold 0.01 it
    // should reject immediately, but loop defensively up to a cap.
    let mut rejected = false;
    for _ in 0..64 {
        let k = Algebra::random_versor(2);
        let v = Algebra::random_versor(2);
        match pipeline.store(&k, &v) {
            Ok(_) => continue,
            Err(MinuetError::CapacityExceeded) => {
                rejected = true;
                break;
            }
            Err(other) => panic!("expected CapacityExceeded, got {other:?}"),
        }
    }
    assert!(rejected, "RejectPolicy never engaged within 64 stores");
}

#[test]
fn capacity_warning_surfaces_before_rejection() {
    // Even when the policy rejects, the store receipt's warning path is
    // independent (SimpleStore always computes it). This pins that the warning
    // surfaces as utilization climbs, separate from the policy gate.
    let store = SimpleStore::<Algebra>::new();
    let mut saw_warning = false;
    for _ in 0..200 {
        let k = Algebra::random_versor(2);
        let v = Algebra::random_versor(2);
        let receipt = store.store(&k, &v).expect("SimpleStore always accepts");
        if receipt.warning.is_some() {
            saw_warning = true;
        }
    }
    assert!(saw_warning, "expected a capacity warning after 200 stores");
}

// ---------------------------------------------------------------------------
// 2. Concurrency
// ---------------------------------------------------------------------------

#[test]
fn concurrent_read_write_sharded_store() {
    // 100 threads: half writing distinct pairs, half reading. ShardedStore
    // holds a parking_lot::RwLock per shard; this exercises real contention.
    // MemoryStore is Send + Sync, so Arc-sharing across a thread::scope is sound.
    let n_shards = 8;
    let store = Arc::new(ShardedStore::<Algebra>::with_shards(n_shards));

    // Pre-populate so readers have something to read.
    for _ in 0..32 {
        let k = Algebra::random_versor(2);
        let v = Algebra::random_versor(2);
        store.store(&k, &v).expect("pre-populate store");
    }

    let before = store.total_items();
    std::thread::scope(|s| {
        // Writers
        for _ in 0..50 {
            let store = Arc::clone(&store);
            s.spawn(move || {
                for _ in 0..20 {
                    let k = Algebra::random_versor(2);
                    let v = Algebra::random_versor(2);
                    let _ = store.store(&k, &v);
                }
            });
        }
        // Readers — retrieval must not panic under concurrent mutation.
        for _ in 0..50 {
            let store = Arc::clone(&store);
            s.spawn(move || {
                for _ in 0..20 {
                    let q = Algebra::random_versor(2);
                    let _ = store.retrieve(&q);
                }
            });
        }
    });

    // All 50 writers × 20 stores should have landed (SimpleStore always accepts).
    let after = store.total_items();
    assert_eq!(
        after - before,
        50 * 20,
        "every concurrent store must be counted"
    );
}

// ---------------------------------------------------------------------------
// 3. Persistence recovery
// ---------------------------------------------------------------------------

#[test]
fn journal_save_load_replays_state() {
    // The persistence-recovery contract: after save → drop in-memory state →
    // reload from disk → replay, the logical state must match what was stored.
    // This is the "crash during checkpoint" resilience path — the journal is
    // the source of truth, optical state is derived/cached.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("stress_journal.bin");

    let mut journal = MemoryJournal::new(
        LeeEncoderConfig {
            carrier_frequency: 0.25,
            carrier_angle: 0.0,
            dimensions: (256, 256),
        },
        CodebookConfig {
            dimensions: (256, 256),
            base_seed: 7,
        },
    );

    // Record a realistic op sequence.
    let sym = amari_holographic::optical::SymbolId::new("STRESS_SYMBOL");
    journal.append(MemoryOp::RegisterSymbol {
        symbol: sym.clone(),
        seed: Some(42),
        timestamp: 1000,
    });
    for i in 0..50 {
        journal.append(MemoryOp::store(
            minuet::optical::SymbolicExpression::symbol(format!("k{i}")),
            minuet::optical::SymbolicExpression::symbol(format!("v{i}")),
            1.0,
        ));
    }

    journal.save(&path).expect("save");

    // Drop the in-memory journal, reload from disk, replay.
    let reloaded = MemoryJournal::load(&path).expect("load");
    let state = reloaded.replay_to_state();

    assert_eq!(state.associations.len(), 50, "all 50 stores replayed");
    assert!(
        state.symbol_seeds.contains_key(&sym),
        "registered symbol survived the round-trip"
    );
}

#[test]
fn checkpointed_memory_restore_recovers_associations() {
    // End-to-end persistence recovery via CheckpointedOpticalMemory: store,
    // checkpoint (flush journal), drop, restore on fresh hardware, verify the
    // associations are present.
    let dir = tempfile::tempdir().expect("tempdir");
    let journal_path = dir.path().join("restore_journal.bin");
    let config = CheckpointConfig {
        journal_path: journal_path.clone(),
        ..Default::default()
    };

    let hardware = MockOpticalHardware::new(42);
    let encoder = LeeEncoderConfig {
        carrier_frequency: 0.25,
        carrier_angle: 0.0,
        dimensions: (256, 256),
    };
    let codebook = CodebookConfig {
        dimensions: (256, 256),
        base_seed: 7,
    };

    use minuet::optical::SymbolicExpression;
    let mut memory = minuet::optical::CheckpointedOpticalMemory::new(
        hardware,
        encoder.clone(),
        codebook.clone(),
        config.clone(),
    )
    .expect("memory constructs");

    for i in 0..10 {
        memory
            .store(
                SymbolicExpression::symbol(format!("k{i}")),
                SymbolicExpression::symbol(format!("v{i}")),
            )
            .expect("store");
    }
    memory.checkpoint().expect("checkpoint flushes the journal");
    let stored = memory.stats().n_associations;
    assert_eq!(stored, 10, "ten associations before restore");
    drop(memory);

    // Restore on fresh hardware — the journal is the source of truth.
    let restored =
        minuet::optical::CheckpointedOpticalMemory::restore(MockOpticalHardware::new(42), config)
            .expect("restore");
    assert_eq!(
        restored.stats().n_associations,
        10,
        "all associations recovered via journal replay"
    );
}

// ---------------------------------------------------------------------------
// 4. Temperature sweep
// ---------------------------------------------------------------------------

#[test]
fn temperature_schedules_are_monotone_across_loads() {
    // The cleanup contract a noisy backend relies on: an annealed schedule's
    // beta sequence must be monotone increasing (soft → hard) regardless of
    // how many items are in the store. We sweep schedule shapes × load levels.
    let shapes: &[(&str, TemperatureSchedule)] = &[
        ("linear", TemperatureSchedule::linear(1.0, 100.0, 50)),
        (
            "exponential",
            TemperatureSchedule::exponential(1.0, 100.0, 50),
        ),
        ("cosine", TemperatureSchedule::cosine(1.0, 100.0, 50)),
    ];

    for load in [0, 1, 10, 50, 100] {
        // Build a store at this load to stress that the schedule is load-
        // independent (it's a pure function of iteration, not memory state).
        let store = SimpleStore::<Algebra>::new();
        for _ in 0..load {
            let k = Algebra::random_versor(2);
            let v = Algebra::random_versor(2);
            let _ = store.store(&k, &v);
        }

        for (name, schedule) in shapes {
            let betas: Vec<f64> = schedule.clone().collect();
            assert_eq!(betas.len(), 50, "{name} @ load {load}: 50 steps");
            // Start ≤ end.
            assert!(
                betas[0] <= *betas.last().unwrap(),
                "{name} @ load {load}: start {:?} must be <= end {:?}",
                betas[0],
                betas.last()
            );
            // Monotone non-decreasing (linear/exp/cosine all anneal soft→hard).
            for w in betas.windows(2) {
                assert!(
                    w[0] <= w[1] + 1e-9,
                    "{name} @ load {load}: schedule must be monotone non-decreasing ({:?})",
                    w
                );
            }
        }
    }
}

#[test]
fn annealed_temperature_drives_resonator_under_load() {
    // End-to-end: an annealed Temperature still produces a converging cleanup
    // across varying codebook sizes (the load dimension for the resonator).
    use minuet::retrieval::ResonatorRetriever;
    use minuet::traits::{RetrievalContext, Retriever};

    for n_symbols in [4, 8, 12] {
        let symbols: Vec<Algebra> = (0..n_symbols).map(|_| Algebra::random_versor(2)).collect();
        let target_idx = n_symbols / 2;

        let retriever = ResonatorRetriever::<Algebra>::new()
            .with_temperature(&Temperature::annealed(1.0, 100.0, 50).expect("annealed"));
        let context = RetrievalContext::default().with_codebook(symbols.clone());

        let result = retriever
            .cleanup(&symbols[target_idx], &context)
            .expect("cleanup");
        assert!(
            result.converged,
            "annealed cleanup should converge at {n_symbols} symbols (conf={})",
            result.confidence
        );
        assert_eq!(
            result.codebook_match,
            Some(target_idx),
            "converged to the correct symbol at load {n_symbols}"
        );
    }
}
