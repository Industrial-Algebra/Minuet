//! Molecular analogy example.
//!
//! Demonstrates using holographic memory for drug discovery-style
//! molecular analogy queries.
//!
//! # Example Query
//!
//! "What might naproxen target, given that aspirin targets COX-2?"
//!
//! This is computed as an analogy: naproxen:X :: aspirin:COX-2

use amari_fusion::holographic::Bindable;
use minuet::{
    binding::Codebook,
    memory::{BasicMemoryStore, MemoryStore, Query},
};

fn main() -> minuet::Result<()> {
    println!("=== Molecular Analogy Example ===\n");

    // Create memory and codebook
    let memory: BasicMemoryStore<f64, 16> = BasicMemoryStore::new();
    let codebook: Codebook<f64, 16> = Codebook::new();

    // Create molecular symbols
    let aspirin = codebook.symbol("aspirin");
    let ibuprofen = codebook.symbol("ibuprofen");
    let naproxen = codebook.symbol("naproxen");
    let acetaminophen = codebook.symbol("acetaminophen");

    // Create target symbols
    let cox1 = codebook.symbol("COX-1");
    let cox2 = codebook.symbol("COX-2");
    let trpv1 = codebook.symbol("TRPV1"); // Acetaminophen's proposed target

    // Create relationship symbol
    let targets = codebook.symbol("targets");

    println!("Created molecular and target symbols.");

    // Store drug-target relationships
    // aspirin -> COX-1, COX-2 (non-selective)
    let aspirin_cox1 = aspirin.bind(&targets).bind(&cox1);
    let aspirin_cox2 = aspirin.bind(&targets).bind(&cox2);
    memory.store(&aspirin, &cox2)?;

    // ibuprofen -> COX-1, COX-2 (non-selective)
    memory.store(&ibuprofen, &cox2)?;

    // acetaminophen -> TRPV1 (different mechanism)
    memory.store(&acetaminophen, &trpv1)?;

    println!("Stored drug-target relationships:");
    println!("  - aspirin -> COX-2");
    println!("  - ibuprofen -> COX-2");
    println!("  - acetaminophen -> TRPV1");
    println!();

    // Query 1: Direct lookup
    println!("Query 1: What does aspirin target?");
    let result = memory.retrieve(&aspirin)?;
    println!("  Confidence: {:.2}", result.confidence);

    // Check similarity to known targets
    let sim_cox2 = result.value.similarity(&cox2);
    let sim_trpv1 = result.value.similarity(&trpv1);
    println!("  Similarity to COX-2: {:.3}", sim_cox2);
    println!("  Similarity to TRPV1: {:.3}", sim_trpv1);
    println!();

    // Query 2: Analogy query
    // "What does naproxen target, if it's like aspirin?"
    // This exploits the structural similarity between NSAIDs
    println!("Query 2: Analogy - naproxen:X :: aspirin:COX-2");
    println!("  (What might naproxen target, if similar to aspirin?)");

    let analogy_query = Query::analogy(aspirin.clone(), cox2.clone(), naproxen.clone());

    let analogy_result = memory.query(analogy_query)?;

    if let Some(top) = analogy_result.top() {
        println!("  Top result similarity: {:.3}", top.similarity);
        let result_sim_cox2 = top.value.similarity(&cox2);
        println!("  Result similarity to COX-2: {:.3}", result_sim_cox2);
    }
    println!();

    // Query 3: Find all drugs with COX-2-like targets
    println!("Query 3: Find drugs with COX-2-like targets");

    let drugs = [
        ("aspirin", &aspirin),
        ("ibuprofen", &ibuprofen),
        ("naproxen", &naproxen),
        ("acetaminophen", &acetaminophen),
    ];

    for (name, drug) in &drugs {
        let result = memory.retrieve(drug)?;
        let sim = result.value.similarity(&cox2);
        println!("  {} -> COX-2 similarity: {:.3}", name, sim);
    }

    println!("\n=== Example Complete ===");

    Ok(())
}
