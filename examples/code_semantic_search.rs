//! Code semantic search example.
//!
//! Demonstrates using holographic memory for semantic code search,
//! finding code fragments by meaning rather than syntax.

use minuet::{
    binding::Codebook,
    domains::symbolic::{AstEncoder, AstNode, CodeSimilarity},
    memory::{BasicMemoryStore, MemoryStore},
};

fn main() -> minuet::Result<()> {
    println!("=== Code Semantic Search Example ===\n");

    // Create memory and encoder
    let memory: BasicMemoryStore<f64, 128> = BasicMemoryStore::new();
    let codebook: Codebook<f64, 128> = Codebook::new();
    let encoder: AstEncoder<f64, 128> = AstEncoder::new();
    let similarity: CodeSimilarity<f64, 128> = CodeSimilarity::new();

    // Define some code patterns
    let patterns = vec![
        (
            "add_integers",
            AstNode::binary("+", AstNode::ident("a"), AstNode::ident("b")),
        ),
        (
            "subtract_integers",
            AstNode::binary("-", AstNode::ident("a"), AstNode::ident("b")),
        ),
        (
            "multiply_values",
            AstNode::binary("*", AstNode::ident("x"), AstNode::ident("y")),
        ),
        (
            "divide_values",
            AstNode::binary("/", AstNode::ident("x"), AstNode::ident("y")),
        ),
        (
            "add_one",
            AstNode::binary("+", AstNode::ident("n"), AstNode::literal("1")),
        ),
        (
            "double",
            AstNode::binary("*", AstNode::ident("n"), AstNode::literal("2")),
        ),
        (
            "function_call",
            AstNode::call("compute", vec![AstNode::ident("x"), AstNode::ident("y")]),
        ),
    ];

    println!("Indexing code patterns...");

    // Store encoded patterns
    for (name, ast) in &patterns {
        let name_sym = codebook.symbol(name);
        let code_enc = encoder.encode(ast);
        memory.store(&code_enc, &name_sym)?;
        println!("  Indexed: {}", name);
    }

    println!("\n--- Semantic Similarity Tests ---\n");

    // Test: find patterns similar to addition
    let query_add = AstNode::binary("+", AstNode::ident("p"), AstNode::ident("q"));
    println!("Query: p + q");
    println!("Finding semantically similar patterns:\n");

    for (name, ast) in &patterns {
        let sim = similarity.similarity(&query_add, ast);
        if sim > 0.1 {
            println!("  {} : similarity = {:.3}", name, sim);
        }
    }

    println!("\n--- Pattern Transformation ---\n");

    // Demonstrate finding refactoring patterns
    let before = AstNode::binary("+", AstNode::ident("x"), AstNode::literal("1"));
    let after = AstNode::call("increment", vec![AstNode::ident("x")]);

    println!("Refactoring pattern:");
    println!("  Before: x + 1");
    println!("  After: increment(x)");

    let before_sim = similarity.similarity(
        &before,
        &AstNode::binary("+", AstNode::ident("n"), AstNode::literal("1")),
    );

    println!("\n  Similarity to 'add_one' pattern: {:.3}", before_sim);

    println!("\n=== Example Complete ===");

    Ok(())
}
