// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Pipeline composition example.
//!
//! This example shows how to compose custom memory pipelines
//! using the PipelineBuilder.
//!
//! Run with: `cargo run --example compose_pipeline`

use minuet::capacity::RejectPolicy;
use minuet::prelude::*;

type Algebra = ProductCliffordAlgebra<32>; // 256 dimensions

fn main() -> MinuetResult<()> {
    println!("=== Pipeline Composition Demo ===\n");

    // Build a custom pipeline
    let pipeline = PipelineBuilder::<Algebra>::new()
        .with_store(ShardedStore::with_shards(4))
        .with_retriever(ResonatorRetriever::new())
        .with_codebook(HashMapCodebook::new())
        .with_capacity_policy(RejectPolicy::with_threshold(0.9))
        .build()?;

    println!("Created pipeline with 4 shards and resonator retrieval\n");

    // Store some associations
    println!("Storing associations...");
    let key1 = pipeline.symbol("dog");
    let value1 = pipeline.symbol("bark");
    pipeline.store(&key1, &value1)?;

    let key2 = pipeline.symbol("cat");
    let value2 = pipeline.symbol("meow");
    pipeline.store(&key2, &value2)?;

    let key3 = pipeline.symbol("cow");
    let value3 = pipeline.symbol("moo");
    pipeline.store(&key3, &value3)?;

    // Retrieve
    println!("\nRetrieving associations:");
    let result = pipeline.retrieve(&key1)?;
    println!("  dog -> (confidence: {:.3})", result.confidence);

    let result = pipeline.retrieve(&key2)?;
    println!("  cat -> (confidence: {:.3})", result.confidence);

    // Show capacity
    let info = pipeline.capacity_info();
    println!("\nPipeline capacity:");
    println!("  Total items: {}", info.total_items);
    println!("  Utilization: {:.1}%", info.utilization * 100.0);

    println!("\n=== Demo Complete ===");
    Ok(())
}
