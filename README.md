# Minuet

[![CI](https://github.com/industrial-algebra/minuet/actions/workflows/ci.yml/badge.svg)](https://github.com/industrial-algebra/minuet/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/minuet.svg)](https://crates.io/crates/minuet)
[![Documentation](https://docs.rs/minuet/badge.svg)](https://docs.rs/minuet)
[![License](https://img.shields.io/crates/l/minuet.svg)](LICENSE)

> "The optical table for holographic computing."

**Minuet** is a Rust toolkit for building holographic memory systems, extending [`amari-holographic`](https://crates.io/crates/amari-holographic) with higher-level abstractions for cognitive memory architectures.

Named after Star Trek's first sentient hologram, Minuet provides memory that participates in cognition rather than merely serving it.

## What is Holographic Memory?

Holographic memory stores information in **superposition** using high-dimensional algebraic representations. Unlike traditional key-value stores:

- **Retrieval is algebraic**: Queries are pattern completions in the same representational space as stored knowledge
- **Relationships are first-class**: Associations are stored as algebraic bindings, enabling analogical queries
- **Graceful degradation**: Memory degrades smoothly under capacity pressure rather than failing catastrophically
- **Compositional**: Complex structures are built from simple binding and bundling operations

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
minuet = "0.1"
amari-holographic = "0.12"
```

### Basic Usage

```rust
use minuet::prelude::*;

// Choose an algebra - ProductCliffordAlgebra<K> has 8*K dimensions
type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions, ~85 item capacity

fn main() -> MinuetResult<()> {
    // Create a simple memory (combines store + codebook)
    let memory = SimpleMemory::<Algebra>::new();

    // Store associations between symbols
    memory.store_symbols("paris", "france")?;
    memory.store_symbols("berlin", "germany")?;
    memory.store_symbols("rome", "italy")?;

    // Recall: given a key, find the associated value
    if let Some((value, confidence)) = memory.recall("paris")? {
        println!("paris -> {} (confidence: {:.2})", value, confidence);
    }

    // Direct algebra operations
    let dog = memory.symbol("dog");
    let bark = memory.symbol("bark");
    memory.store(&dog, &bark)?;

    Ok(())
}
```

### Pipeline Composition

For more control, compose custom pipelines:

```rust
use minuet::prelude::*;
use minuet::capacity::RejectPolicy;

type Algebra = ProductCliffordAlgebra<32>; // 256 dimensions

fn main() -> MinuetResult<()> {
    // Build a custom pipeline
    let pipeline = PipelineBuilder::<Algebra>::new()
        .with_store(ShardedStore::with_shards(4))      // 4 shards for ~4x capacity
        .with_retriever(ResonatorRetriever::new())     // Cleanup via resonator network
        .with_codebook(HashMapCodebook::new())         // Symbol vocabulary
        .with_capacity_policy(RejectPolicy::with_threshold(0.9))
        .build()?;

    // Use the pipeline
    let key = pipeline.symbol("query");
    let value = pipeline.symbol("result");
    pipeline.store(&key, &value)?;

    let result = pipeline.retrieve(&key)?;
    println!("Retrieved with confidence: {:.2}", result.confidence);

    Ok(())
}
```

## Core Concepts

### Binding Algebras

Minuet is generic over any `BindingAlgebra` from `amari-holographic`. The algebra provides:

| Operation | Symbol | Description |
|-----------|--------|-------------|
| **Bind** | `a.bind(&b)` | Create association (dissimilar to inputs) |
| **Bundle** | `a.bundle(&b, β)` | Superpose (similar to inputs) |
| **Unbind** | `key.unbind(&trace)` | Retrieve associated value |
| **Similarity** | `a.similarity(&b)` | Measure closeness [-1, 1] |

Available algebras:

| Type | Dimensions | Use Case |
|------|------------|----------|
| `ProductCliffordAlgebra<K>` | 8×K | General purpose, recommended |
| `Cl3` | 8 | Small/embedded systems |
| `FHRRAlgebra<D>` | D | Frequency domain operations |
| `MAPAlgebra<D>` | D | Binary/bipolar systems |

### Memory Traces

A **trace** stores items in superposition:

```rust
use minuet::store::DenseTrace;

let mut trace = DenseTrace::<Algebra>::new();

// Add items (they superpose)
trace.add(&item1, 1.0);  // weight = 1.0
trace.add(&item2, 1.0);

// Query similarity
let sim = trace.similarity(&item1);  // High similarity

// Unbind to retrieve
let retrieved = trace.unbind(&key);
```

### Memory Stores

Stores manage one or more traces:

| Store | Description |
|-------|-------------|
| `SimpleStore` | Single trace, minimal overhead |
| `ShardedStore` | Hash-sharded across N traces for N× capacity |

```rust
// Simple store for small workloads
let simple = SimpleStore::<Algebra>::new();

// Sharded store for larger capacity
let sharded = ShardedStore::<Algebra>::with_shards(8);
```

### Codebooks

Codebooks provide consistent symbol-to-vector mappings:

```rust
use minuet::encoding::HashMapCodebook;

let codebook = HashMapCodebook::<Algebra>::new();

// Same name always returns same vector
let v1 = codebook.symbol("hello");
let v2 = codebook.symbol("hello");
assert!(v1.similarity(&v2) > 0.99);

// Find closest symbol to a vector
if let Some((name, similarity)) = codebook.closest(&query) {
    println!("Closest: {} ({:.2})", name, similarity);
}
```

### Retrievers

Retrievers clean up noisy retrieval results:

| Retriever | Description |
|-----------|-------------|
| `DirectRetriever` | Return raw result (no cleanup) |
| `ResonatorRetriever` | Iterative cleanup via resonator network |

```rust
use minuet::retrieval::ResonatorRetriever;

// Resonator with custom settings
let retriever = ResonatorRetriever::<Algebra>::new()
    .initial_temperature(1.0)   // Start soft
    .final_temperature(100.0)   // End hard
    .max_iterations(50);
```

### Capacity Management

Holographic memory has finite capacity based on dimensions:

| Algebra | Dimensions | Approximate Capacity |
|---------|------------|---------------------|
| `ProductCliffordAlgebra<16>` | 128 | ~23 items |
| `ProductCliffordAlgebra<32>` | 256 | ~46 items |
| `ProductCliffordAlgebra<64>` | 512 | ~85 items |
| `ProductCliffordAlgebra<128>` | 1024 | ~147 items |

Capacity scales as **O(D / ln D)** where D is dimension.

For larger workloads, use `ShardedStore`:

```rust
// 8 shards × 85 items ≈ 680 item capacity
let store = ShardedStore::<ProductCliffordAlgebra<64>>::with_shards(8);
```

## Architecture

```
minuet/
├── src/
│   ├── lib.rs           # Re-exports and prelude
│   ├── traits.rs        # Core trait definitions
│   ├── error.rs         # Error types
│   ├── store/           # Memory storage
│   │   ├── trace.rs     # DenseTrace - fundamental storage unit
│   │   ├── simple.rs    # SimpleStore - single-trace store
│   │   └── sharded.rs   # ShardedStore - hash-sharded store
│   ├── encoding/        # Symbol encoding
│   │   └── codebook.rs  # HashMapCodebook
│   ├── retrieval/       # Cleanup strategies
│   │   ├── direct.rs    # DirectRetriever
│   │   └── resonator_retriever.rs
│   ├── capacity/        # Capacity management
│   │   └── mod.rs       # RejectPolicy, AcceptAllPolicy
│   ├── pipeline/        # Composition
│   │   └── builder.rs   # PipelineBuilder
│   └── reference/       # Reference implementations
│       └── simple_memory.rs  # SimpleMemory
├── examples/
│   ├── simple_memory.rs    # Basic usage
│   └── compose_pipeline.rs # Pipeline composition
└── tests/
```

## Examples

### Simple Memory

Basic store-and-recall operations:

```bash
cargo run --example simple_memory
```

### Pipeline Composition

Building custom pipelines with sharding and resonator cleanup:

```bash
cargo run --example compose_pipeline
```

## Feature Flags

```toml
[dependencies]
minuet = { version = "0.1", features = ["parallel"] }
```

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | Standard features | `std` |
| `std` | Standard library support | - |
| `parallel` | Rayon parallelism | `rayon` |
| `serde` | Serialization | `serde`, `bincode` |
| `persistence` | RocksDB storage | `rocksdb`, `serde` |
| `async` | Async support | `tokio` |
| `full` | All features | all above |

## Algebraic Guarantees

The underlying `BindingAlgebra` satisfies:

1. **Identity**: `x.bind(&A::identity()) = x`
2. **Inverse**: `x.bind(&x.inverse()?) ≈ A::identity()`
3. **Dissimilarity**: `a.bind(&b)` is dissimilar to both `a` and `b`
4. **Bundle Similarity**: `a.bundle(&b, β)?` is similar to both `a` and `b`
5. **Distributivity**: `a.bind(&b.bundle(&c, β)?) ≈ a.bind(&b).bundle(&a.bind(&c), β)?`

## Performance Considerations

- **Dimension Choice**: Higher dimensions = more capacity but slower operations
- **Sharding**: Use `ShardedStore` when single-trace capacity is insufficient
- **Retriever**: `DirectRetriever` is fastest; `ResonatorRetriever` improves accuracy
- **Bundling Temperature**: β=1.0 (soft) preserves more information; β=∞ (hard) is faster

## Use Cases

| Domain | Application |
|--------|-------------|
| **Cognitive Agents** | Working memory with associative recall |
| **Knowledge Graphs** | Relationship storage and analogical queries |
| **Semantic Search** | Content-addressable retrieval |
| **Neurosymbolic AI** | Symbol grounding with compositional generalization |
| **Robotics** | Motor primitive composition |

## What Minuet Is Not

- ❌ A replacement for vector databases at scale (millions of items)
- ❌ A general-purpose key-value store
- ❌ An embedding similarity search engine

Minuet excels at **small-to-medium associative memories** where algebraic structure matters.

## Testing

```bash
# Run all tests
cargo test

# Run with all features
cargo test --all-features

# Run examples
cargo run --example simple_memory
cargo run --example compose_pipeline
```

## Minimum Supported Rust Version

Rust 1.78 or later.

## License

MIT OR Apache-2.0

## References

- [amari-holographic](https://crates.io/crates/amari-holographic) - Core binding algebras
- Holographic Reduced Representations (Plate, 1995)
- Hyperdimensional Computing (Kanerva, 2009)
- Vector Symbolic Architectures (Gayler, 2003)

## Contributing

Contributions welcome! Please see the [GitHub repository](https://github.com/industrial-algebra/minuet) for issues and pull requests.
