# Minuet: A Holographic Database

> "A mind graft, not a translation layer."

**Minuet** is a holographic database built on `amari-fusion`'s tropical-dual-Clifford algebra. Named after Star Trek's first sentient hologram, Minuet provides memory that participates in cognition rather than merely serving it.

## Core Proposition

Retrieval is a native algebraic operation, not index lookup with a translation layer. Queries are pattern completions in the same representational space as stored knowledge.

## Features

- **Compositional associative memory** where relationships are first-class
- **Analogical queries** like "find X related to Y as A is related to B" as single operations
- **Graceful degradation** under noise, partial queries, and capacity pressure
- **Type-safe algebra** with phantom types tracking invertibility, normalization, and grade
- **Formal verification** support via Creusot contracts
- **High-precision numerics** with f64 and optional BigFloat support

## Quick Start

```rust
use minuet::prelude::*;

// Create a memory store
let memory: BasicMemoryStore<f64, 128> = BasicMemoryStore::new();
let codebook: Codebook<f64, 128> = Codebook::new();

// Create symbols
let paris = codebook.symbol("paris");
let france = codebook.symbol("france");
let berlin = codebook.symbol("berlin");

// Store: paris is associated with france
memory.store(&paris, &france)?;

// Direct retrieval
let result = memory.retrieve(&paris)?;

// Analogy query: what is to berlin as france is to paris?
let query = Query::analogy(paris, france, berlin);
let analogy_result = memory.query(query)?;
```

## Architecture

```
minuet/
├── src/
│   ├── lib.rs              # Main library entry
│   ├── error.rs            # Error types with phantom markers
│   ├── memory/             # Core holographic storage
│   │   ├── trace.rs        # Holographic trace (superposition)
│   │   ├── store.rs        # MemoryStore trait + implementations
│   │   ├── query.rs        # Query builder (key, analogy, transform)
│   │   └── capacity.rs     # SNR tracking and capacity estimation
│   ├── binding/            # Algebraic operations
│   │   ├── algebra.rs      # Extended Bindable trait
│   │   ├── codebook.rs     # Symbol vocabularies
│   │   └── transform.rs    # Reified transformations
│   ├── retrieval/          # Cleanup and attribution
│   │   ├── resonator.rs    # Iterative cleanup networks
│   │   ├── attribution.rs  # Provenance tracking
│   │   └── temperature.rs  # Soft/hard retrieval control
│   ├── parallel/           # Parallel operations
│   │   ├── batch.rs        # Rayon-based batch ops
│   │   ├── sharded.rs      # Sharded memory for scale
│   │   └── merge.rs        # Parallel trace merging
│   ├── persistence/        # Durability (optional)
│   ├── gpu/                # GPU acceleration (optional)
│   └── domains/            # Domain-specific encoders
│       ├── molecular.rs    # SMILES, fingerprints
│       ├── geometric.rs    # SE(3) motors
│       └── symbolic.rs     # Code ASTs
├── tests/
│   └── algebraic_laws.rs   # Property-based tests
├── benches/                # Criterion benchmarks
└── examples/               # Usage examples
```

## Capacity Model

Holographic memory has capacity O(DIM / log DIM):

| Dimension | Approx. Capacity |
|-----------|------------------|
| 64        | ~10 items        |
| 256       | ~45 items        |
| 1024      | ~150 items       |
| 4096      | ~500 items       |

For larger capacities, use `ShardedMemory` which distributes across multiple traces.

## Features

```toml
[dependencies]
minuet = { version = "0.1", features = ["default"] }

# Optional features:
# contracts    - Creusot formal verification
# high-precision - BigFloat numerics
# gpu          - WGPU acceleration
# persistence  - RocksDB durability
# distributed  - Tokio/Tonic networking
# full         - All optional features
```

## Examples

### Molecular Analogy

```bash
cargo run --example molecular_analogy
```

Demonstrates drug-target relationship queries.

### Code Semantic Search

```bash
cargo run --example code_semantic_search
```

Semantic code search using AST encoding.

### Motor Primitives

```bash
cargo run --example motor_primitives
```

SE(3) motor composition for robotics.

### Working Memory Agent

```bash
cargo run --example working_memory_agent
```

Context-dependent associative memory for AI agents.

## Benchmarks

```bash
cargo bench
```

Runs benchmarks for:
- Binding throughput (sequential vs parallel)
- Retrieval latency at various loads
- Capacity scaling
- Parallel operation speedup

## Testing

```bash
# Unit tests
cargo test

# Property-based algebraic law tests
cargo test --test algebraic_laws

# With contracts (requires Creusot)
cargo test --features contracts
```

## Algebraic Guarantees

The binding algebra satisfies:

1. **Identity**: `x ⊛ I ≈ x`
2. **Inverse**: `x ⊛ x⁻¹ ≈ I`
3. **Associativity**: `(a ⊛ b) ⊛ c ≈ a ⊛ (b ⊛ c)`
4. **Dissimilarity**: `a ⊛ b` is dissimilar to both `a` and `b`
5. **Distributivity**: `a ⊛ (b ⊕ c) ≈ (a ⊛ b) ⊕ (a ⊛ c)`

These are verified by property-based tests in `tests/algebraic_laws.rs`.

## Intended Use Cases

| Domain | Key Operation |
|--------|---------------|
| Drug discovery | Molecular analogy: "X relates to target T as drug D relates to its target" |
| Robotics | Motor primitive composition with native SE(3) geometry |
| Code understanding | Semantic search and refactoring-as-transformation |
| Legal reasoning | Precedent retrieval by analogical structure |
| Multi-agent systems | Mergeable world models, theory of mind |
| Neurosymbolic AI | Symbol grounding with compositional generalization |

## What Minuet Is Not

- A replacement for vector databases at scale
- A general-purpose DBMS
- An embedding similarity search engine

## License

MIT OR Apache-2.0

## References

- [amari-fusion](https://github.com/justinelliottcobb/Amari) - Tropical-dual-Clifford algebra
- [Creusot](https://github.com/creusot-rs/creusot) - Formal verification for Rust
- Holographic Reduced Representations (Plate, 1995)
- Hyperdimensional Computing (Kanerva, 2009)
