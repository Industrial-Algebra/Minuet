// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Simple memory example demonstrating the basic Minuet API.
//!
//! This example shows how to:
//! - Create a simple holographic memory
//! - Store and retrieve key-value associations
//! - Query the symbol codebook
//!
//! Run with: `cargo run --example simple_memory`

use minuet::prelude::*;

type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions

fn main() -> MinuetResult<()> {
    println!("=== Simple Holographic Memory Demo ===\n");

    // Create a simple memory
    let memory = SimpleMemory::<Algebra>::new();

    // Store some capital city associations
    println!("Storing country-capital associations...");
    memory.store_symbols("france", "paris")?;
    memory.store_symbols("germany", "berlin")?;
    memory.store_symbols("spain", "madrid")?;
    memory.store_symbols("italy", "rome")?;

    println!("Stored {} associations\n", memory.item_count());

    // Recall capitals
    println!("Recalling capitals:");
    for country in &["france", "germany", "spain", "italy"] {
        if let Some((capital, confidence)) = memory.recall(country)? {
            println!(
                "  {} -> {} (confidence: {:.3})",
                country, capital, confidence
            );
        }
    }

    // Show symbol count
    println!(
        "\nSymbol codebook contains {} symbols",
        memory.symbol_count()
    );

    // Demonstrate capacity info
    let info = memory.capacity_info();
    println!("\nCapacity info:");
    println!("  Total items: {}", info.total_items);
    println!("  Theoretical capacity: {}", info.theoretical_capacity);
    println!("  Estimated SNR: {:.2}", info.estimated_snr);
    println!("  Utilization: {:.1}%", info.utilization * 100.0);

    println!("\n=== Demo Complete ===");
    Ok(())
}
