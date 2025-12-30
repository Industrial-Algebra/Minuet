//! Optical Memory Demo
//!
//! Demonstrates the optical backend with checkpoint-based persistence.
//!
//! Run with: cargo run --example optical_memory_demo --features optical

use std::path::PathBuf;
use std::time::Duration;

use minuet::optical::{
    CheckpointConfig, CheckpointedOpticalMemory, MockOpticalHardware, OpticalHardware,
    SymbolicExpression,
};

use amari_holographic::optical::{CodebookConfig, LeeEncoderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Optical Memory Demo ===\n");

    // Configuration
    let encoder_config = LeeEncoderConfig {
        carrier_frequency: 0.25,
        carrier_angle: 0.0,
        dimensions: (256, 256),
    };

    let codebook_config = CodebookConfig {
        dimensions: (256, 256),
        base_seed: 12345,
    };

    let checkpoint_config = CheckpointConfig {
        interval: Duration::from_secs(300), // 5 minutes
        max_ops_before_compact: 10_000,
        journal_path: PathBuf::from("/tmp/optical_memory_demo.bin"),
    };

    // Create mock hardware (simulates DMD + MMF system)
    let hardware = MockOpticalHardware::new(42);
    println!("Created mock hardware: {}", hardware.id());
    println!("  Dimensions: {:?}", hardware.dimensions());
    println!("  Modes: {}\n", hardware.n_modes());

    // Create checkpointed optical memory
    let mut memory = CheckpointedOpticalMemory::new(
        hardware,
        encoder_config,
        codebook_config,
        checkpoint_config.clone(),
    )?;

    println!("Hardware info:");
    let info = memory.hardware_info();
    println!("  ID: {}", info.id);
    println!("  Dimensions: {:?}", info.dimensions);
    println!("  Ready: {}", info.is_ready);
    println!("  Calibrated: {}\n", info.is_calibrated);

    // Store some associations using role-filler bindings
    println!("Storing associations...");

    memory.store(
        SymbolicExpression::role_filler("AGENT", "John"),
        SymbolicExpression::role_filler("ACTION", "running"),
    )?;

    memory.store(
        SymbolicExpression::role_filler("AGENT", "Mary"),
        SymbolicExpression::role_filler("ACTION", "reading"),
    )?;

    memory.store(
        SymbolicExpression::role_filler("OBJECT", "book"),
        SymbolicExpression::role_filler("LOCATION", "library"),
    )?;

    // Also store some simple symbol associations
    memory.store(
        SymbolicExpression::symbol("cat"),
        SymbolicExpression::symbol("meow"),
    )?;

    memory.store(
        SymbolicExpression::symbol("dog"),
        SymbolicExpression::symbol("bark"),
    )?;

    let stats = memory.stats();
    println!("Stored {} associations\n", stats.n_associations);

    // Retrieve associations
    println!("Retrieving associations...");

    if let Some(result) = memory.retrieve(&SymbolicExpression::role_filler("AGENT", "John"))? {
        println!(
            "  John's action: {:?} (similarity: {:.3})",
            result.value, result.similarity
        );
    }

    if let Some(result) = memory.retrieve(&SymbolicExpression::symbol("cat"))? {
        println!(
            "  Cat says: {:?} (similarity: {:.3})",
            result.value, result.similarity
        );
    }

    if let Some(result) = memory.retrieve(&SymbolicExpression::symbol("dog"))? {
        println!(
            "  Dog says: {:?} (similarity: {:.3})",
            result.value, result.similarity
        );
    }

    println!();

    // Demonstrate decay
    println!("Applying memory decay (factor 0.8)...");
    memory.decay(0.8)?;

    let stats = memory.stats();
    println!("Associations after decay: {}\n", stats.n_associations);

    // Checkpoint
    println!("Creating checkpoint...");
    memory.checkpoint()?;
    println!(
        "Checkpoint saved to: {:?}\n",
        checkpoint_config.journal_path
    );

    // Simulate restart with same hardware
    println!("Simulating restart with same hardware...");
    drop(memory);

    let same_hardware = MockOpticalHardware::new(42);
    let mut restored =
        CheckpointedOpticalMemory::restore(same_hardware, checkpoint_config.clone())?;

    println!("Memory restored!");
    let stats = restored.stats();
    println!("  Associations: {}", stats.n_associations);
    println!("  Symbols: {}", stats.n_symbols);

    // Verify we can still retrieve
    if let Some(result) = restored.retrieve(&SymbolicExpression::symbol("cat"))? {
        println!(
            "  Cat still says: {:?} (similarity: {:.3})\n",
            result.value, result.similarity
        );
    }

    // Simulate restart with different hardware
    println!("Simulating restart with DIFFERENT hardware...");
    drop(restored);

    let different_hardware = MockOpticalHardware::new(999); // Different seed = different T-matrix
    println!("New hardware ID: {}", different_hardware.id());

    let mut migrated = CheckpointedOpticalMemory::restore(different_hardware, checkpoint_config)?;

    println!("Memory migrated to new hardware!");
    let stats = migrated.stats();
    println!("  Associations: {}", stats.n_associations);

    // Verify we can still retrieve after migration
    if let Some(result) = migrated.retrieve(&SymbolicExpression::symbol("dog"))? {
        println!(
            "  Dog still says: {:?} (similarity: {:.3})\n",
            result.value, result.similarity
        );
    }

    println!("=== Demo Complete ===");
    Ok(())
}
