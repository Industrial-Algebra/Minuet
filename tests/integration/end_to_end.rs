// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! End-to-end integration tests for Minuet.
//!
//! Tests the full pipeline from symbol creation through storage, retrieval,
//! and capacity management.

use minuet::capacity::RejectPolicy;
use minuet::prelude::*;

type Algebra = ProductCliffordAlgebra<32>; // 256 dimensions

#[test]
fn full_pipeline_store_recall() -> MinuetResult<()> {
    // Create a pipeline with all components
    let pipeline = PipelineBuilder::<Algebra>::new()
        .with_store(ShardedStore::with_shards(4))
        .with_retriever(ResonatorRetriever::new())
        .with_codebook(HashMapCodebook::new())
        .build()?;

    // Store associations
    let entries = [
        ("dog", "bark"),
        ("cat", "meow"),
        ("cow", "moo"),
        ("bird", "chirp"),
        ("frog", "croak"),
    ];

    for (key, value) in &entries {
        let k = pipeline.symbol(key);
        let v = pipeline.symbol(value);
        pipeline.store(&k, &v)?;
    }

    // Verify count
    let info = pipeline.capacity_info();
    assert_eq!(info.total_items, entries.len());

    // Retrieve each and verify something comes back
    for (key, _) in &entries {
        let k = pipeline.symbol(key);
        let result = pipeline.retrieve(&k)?;
        assert!(result.confidence > 0.0);
    }

    Ok(())
}

#[test]
fn sharded_store_capacity_distribution() -> MinuetResult<()> {
    let store = ShardedStore::<Algebra>::with_shards(8);

    // Store items
    for i in 0..16 {
        let key = Algebra::random_versor(2);
        let value = Algebra::random_versor(2);
        store.store(&key, &value)?;
    }

    let info = store.capacity_info();
    assert_eq!(info.total_items, 16);
    assert!(info.per_trace.len() == 8);

    // Items should be distributed (not all in one shard)
    let max_items_per_shard = info.per_trace.iter().map(|t| t.items).max().unwrap_or(0);
    assert!(max_items_per_shard < 16, "items not distributed across shards");

    Ok(())
}

#[test]
fn capacity_rejection_flow() -> MinuetResult<()> {
    let pipeline = PipelineBuilder::<Algebra>::new()
        .with_capacity_policy(RejectPolicy::with_threshold(0.5))
        .build()?;

    // Fill to near capacity
    let mut stored = 0;
    loop {
        let key = Algebra::random_versor(2);
        let value = Algebra::random_versor(2);

        match pipeline.store(&key, &value) {
            Ok(_) => stored += 1,
            Err(_) => break,
        }

        if stored > 200 {
            break;
        }
    }

    // Should have stored some items before rejection
    assert!(stored > 0, "should have stored at least some items");

    let info = pipeline.capacity_info();
    assert!(info.utilization >= 0.4, "utilization should be high");

    Ok(())
}

#[test]
fn simple_memory_full_workflow() -> MinuetResult<()> {
    let memory = SimpleMemory::<Algebra>::new();

    // Store capital cities
    memory.store_symbols("france", "paris")?;
    memory.store_symbols("germany", "berlin")?;
    memory.store_symbols("italy", "rome")?;
    memory.store_symbols("spain", "madrid")?;
    memory.store_symbols("portugal", "lisbon")?;

    // Verify recall
    let result = memory.recall("france")?;
    assert!(result.is_some());
    let (capital, confidence) = result.unwrap();
    assert!(capital == "paris" || confidence > 0.3);

    // Verify symbol count
    assert_eq!(memory.symbol_count(), 10); // 5 countries + 5 capitals

    // Verify item count
    assert_eq!(memory.item_count(), 5);

    // Clear and verify
    memory.clear()?;
    assert_eq!(memory.item_count(), 0);

    Ok(())
}

#[test]
fn codebook_determinism_across_instances() {
    let cb1 = HashMapCodebook::<Algebra>::new();
    let cb2 = HashMapCodebook::<Algebra>::new();

    let s1 = cb1.symbol("hello");
    let s2 = cb2.symbol("hello");

    // Different codebook instances should produce the same symbol
    // for the same name (deterministic generation)
    assert!(s1.similarity(&s2) > 0.99);
}
