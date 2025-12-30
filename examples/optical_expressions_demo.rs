//! Symbolic Expressions Demo
//!
//! Demonstrates the SymbolicExpression type for building hardware-independent
//! memory representations. Shows symbols, bindings, and bundles.
//!
//! Run with: cargo run --example optical_expressions_demo --features optical

use minuet::optical::SymbolicExpression;

fn main() {
    println!("=== Symbolic Expressions Demo ===\n");

    // --- Atomic Symbols ---
    println!("--- Atomic Symbols ---");
    let cat = SymbolicExpression::symbol("cat");
    let dog = SymbolicExpression::symbol("dog");
    let agent = SymbolicExpression::symbol("AGENT");

    println!("  cat: {}", cat);
    println!("  dog: {}", dog);
    println!("  AGENT: {}", agent);
    println!("  Is symbol? {}", cat.is_symbol());
    println!();

    // --- Bindings (Associations) ---
    println!("--- Bindings (Role-Filler Associations) ---");

    // Manual binding
    let agent_john = SymbolicExpression::bind(
        SymbolicExpression::symbol("AGENT"),
        SymbolicExpression::symbol("John"),
    );
    println!("  Manual: {}", agent_john);

    // Convenience method
    let agent_mary = SymbolicExpression::role_filler("AGENT", "Mary");
    println!("  Convenience: {}", agent_mary);

    // Nested binding (scene representation)
    let scene = SymbolicExpression::bind(
        SymbolicExpression::role_filler("AGENT", "John"),
        SymbolicExpression::bind(
            SymbolicExpression::role_filler("ACTION", "chase"),
            SymbolicExpression::role_filler("PATIENT", "Mary"),
        ),
    );
    println!("  Nested scene: {}", scene);
    println!("  Is binding? {}", scene.is_bind());
    println!();

    // --- Bundles (Superpositions) ---
    println!("--- Bundles (Weighted Superpositions) ---");

    // Weighted bundle
    let weighted = SymbolicExpression::bundle(vec![
        (1.0, SymbolicExpression::symbol("primary")),
        (0.5, SymbolicExpression::symbol("secondary")),
        (0.25, SymbolicExpression::symbol("tertiary")),
    ]);
    println!("  Weighted: {}", weighted);

    // Uniform bundle
    let uniform = SymbolicExpression::bundle_uniform(vec![
        SymbolicExpression::symbol("red"),
        SymbolicExpression::symbol("green"),
        SymbolicExpression::symbol("blue"),
    ]);
    println!("  Uniform: {}", uniform);

    // Bundle of bindings (multiple facts)
    let facts = SymbolicExpression::bundle_uniform(vec![
        SymbolicExpression::role_filler("CAT", "meow"),
        SymbolicExpression::role_filler("DOG", "bark"),
        SymbolicExpression::role_filler("COW", "moo"),
    ]);
    println!("  Facts bundle: {}", facts);
    println!("  Is bundle? {}", facts.is_bundle());
    println!();

    // --- Expression Analysis ---
    println!("--- Expression Analysis ---");

    let complex = SymbolicExpression::bind(
        SymbolicExpression::bundle(vec![
            (1.0, SymbolicExpression::symbol("A")),
            (0.5, SymbolicExpression::symbol("B")),
        ]),
        SymbolicExpression::bind(
            SymbolicExpression::symbol("X"),
            SymbolicExpression::symbol("Y"),
        ),
    );

    println!("  Expression: {}", complex);
    println!("  Node count: {}", complex.node_count());
    println!("  Tree depth: {}", complex.depth());

    let symbols = complex.referenced_symbols();
    print!("  Referenced symbols:");
    for sym in &symbols {
        print!(" {}", sym);
    }
    println!("\n");

    // --- Serialization Roundtrip ---
    println!("--- Serialization ---");

    let original = SymbolicExpression::bind(
        SymbolicExpression::role_filler("SUBJECT", "Alice"),
        SymbolicExpression::bundle(vec![
            (0.8, SymbolicExpression::role_filler("VERB", "loves")),
            (0.2, SymbolicExpression::role_filler("VERB", "likes")),
        ]),
    );

    let json = serde_json::to_string_pretty(&original).unwrap();
    println!("  Original: {}", original);
    println!("  JSON:\n{}", json);

    let restored: SymbolicExpression = serde_json::from_str(&json).unwrap();
    println!("  Restored: {}", restored);
    println!("  Roundtrip OK: {}", original == restored);

    println!("\n=== Demo Complete ===");
}
