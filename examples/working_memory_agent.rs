//! Working memory agent example.
//!
//! Demonstrates using holographic memory as a working memory
//! substrate for an AI agent, enabling:
//!
//! - Short-term memory with capacity limits
//! - Associative recall
//! - Context-dependent retrieval
//! - Memory consolidation

use minuet::{
    binding::{Codebook, Transform},
    memory::{BasicMemoryStore, CapacityInfo, MemoryStore, Query},
    retrieval::{Resonator, ResonatorConfig, Temperature},
};

use amari_fusion::{holographic::Bindable, TropicalDualClifford};

/// A simple working memory agent.
struct WorkingMemoryAgent {
    /// Short-term memory (high capacity, fast decay).
    short_term: BasicMemoryStore<f64, 128>,
    /// Long-term memory (lower capacity per chunk, stable).
    long_term: BasicMemoryStore<f64, 256>,
    /// Symbol vocabulary.
    codebook: Codebook<f64, 128>,
    /// Current context vector.
    context: TropicalDualClifford<f64, 128>,
}

impl WorkingMemoryAgent {
    fn new() -> Self {
        Self {
            short_term: BasicMemoryStore::new(),
            long_term: BasicMemoryStore::new(),
            codebook: Codebook::new(),
            context: TropicalDualClifford::binding_identity(),
        }
    }

    /// Perceive and store a new observation.
    fn perceive(&self, observation: &str, value: &str) -> minuet::Result<()> {
        let obs_sym = self.codebook.symbol(observation);
        let val_sym = self.codebook.symbol(value);

        // Bind with current context
        let contextualized = self.context.bind(&obs_sym);

        self.short_term.store(&contextualized, &val_sym)?;

        Ok(())
    }

    /// Recall from memory given a cue.
    fn recall(&self, cue: &str) -> Option<(String, f64)> {
        let cue_sym = self.codebook.symbol(cue);
        let contextualized = self.context.bind(&cue_sym);

        if let Ok(result) = self.short_term.retrieve(&contextualized) {
            // Find nearest symbol
            self.codebook.nearest(&result.value)
        } else {
            None
        }
    }

    /// Update the current context.
    fn set_context(&mut self, context: &str) {
        self.context = self.codebook.symbol(context);
    }

    /// Get working memory status.
    fn status(&self) -> CapacityInfo {
        self.short_term.capacity()
    }
}

fn main() -> minuet::Result<()> {
    println!("=== Working Memory Agent Example ===\n");

    let mut agent = WorkingMemoryAgent::new();

    // Scenario: Agent learning about a room
    println!("--- Scene: Kitchen ---\n");

    agent.set_context("kitchen");

    agent.perceive("color:walls", "white")?;
    agent.perceive("appliance:large", "refrigerator")?;
    agent.perceive("appliance:cooking", "stove")?;
    agent.perceive("furniture:seating", "chair")?;
    agent.perceive("window:count", "two")?;

    println!("Stored observations about kitchen.");
    println!("Memory status: {} items", agent.status().item_count);

    // Test recall in same context
    println!("\n--- Recall Test (same context: kitchen) ---\n");

    let queries = ["color:walls", "appliance:large", "furniture:seating"];
    for query in queries {
        if let Some((answer, confidence)) = agent.recall(query) {
            println!(
                "  Q: {} -> A: {} (confidence: {:.2})",
                query, answer, confidence
            );
        } else {
            println!("  Q: {} -> not found", query);
        }
    }

    // Change context and test recall
    println!("\n--- Context Change: Bedroom ---\n");

    agent.set_context("bedroom");

    agent.perceive("color:walls", "blue")?;
    agent.perceive("furniture:sleeping", "bed")?;
    agent.perceive("furniture:storage", "dresser")?;

    println!("Stored observations about bedroom.");

    // Recall in new context
    println!("\n--- Recall Test (context: bedroom) ---\n");

    for query in ["color:walls", "furniture:seating", "furniture:sleeping"] {
        if let Some((answer, confidence)) = agent.recall(query) {
            println!(
                "  Q: {} -> A: {} (confidence: {:.2})",
                query, answer, confidence
            );
        } else {
            println!("  Q: {} -> not found", query);
        }
    }

    // Switch back to kitchen context
    println!("\n--- Context Switch Back: Kitchen ---\n");

    agent.set_context("kitchen");

    for query in ["color:walls", "appliance:large"] {
        if let Some((answer, confidence)) = agent.recall(query) {
            println!(
                "  Q: {} -> A: {} (confidence: {:.2})",
                query, answer, confidence
            );
        }
    }

    // Show final status
    println!("\n--- Final Memory Status ---\n");

    let status = agent.status();
    println!("  Items stored: {}", status.item_count);
    println!("  Utilization: {:.1}%", status.utilization * 100.0);
    println!("  Estimated SNR: {:.2}", status.estimated_snr);

    println!("\n=== Example Complete ===");

    Ok(())
}
