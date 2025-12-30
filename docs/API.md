# Minuet API Reference

> "The optical table for holographic computing."

Minuet is a Rust toolkit extending `amari-holographic` with higher-level abstractions for building cognitive memory systems.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Module Reference](#module-reference)
  - [traits](#traits-module)
  - [store](#store-module)
  - [encoding](#encoding-module)
  - [retrieval](#retrieval-module)
  - [capacity](#capacity-module)
  - [pipeline](#pipeline-module)
  - [reference](#reference-module)
  - [optical](#optical-module-feature-gated)
- [Error Handling](#error-handling)
- [Feature Flags](#feature-flags)

---

## Quick Start

```rust
use minuet::prelude::*;
use amari_holographic::ProductCliffordAlgebra;

type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions

fn main() -> MinuetResult<()> {
    // Create a simple memory
    let memory = SimpleMemory::<Algebra>::new();

    // Store associations
    memory.store_symbols("france", "paris")?;
    memory.store_symbols("germany", "berlin")?;

    // Recall
    if let Some((capital, confidence)) = memory.recall("france")? {
        println!("Capital: {} (confidence: {:.3})", capital, confidence);
    }

    Ok(())
}
```

---

## Core Concepts

### Holographic Memory

Holographic memory stores information in superposition. Multiple key-value associations are bundled into a single high-dimensional vector (the "trace"). Retrieval uses algebraic unbinding to recover associated values.

### Binding Algebras

All Minuet types are generic over `A: BindingAlgebra`. The algebra provides:
- **Binding** (`bind`): Associate two elements (key ⊗ value)
- **Unbinding** (`unbind`): Recover associated element (trace ⊘ key → value)
- **Bundling** (`bundle`): Superposition of multiple elements
- **Similarity**: Cosine-like similarity measure

Common algebra choices:
| Type | Dimensions | Capacity |
|------|------------|----------|
| `ProductCliffordAlgebra<32>` | 256 | ~46 items |
| `ProductCliffordAlgebra<64>` | 512 | ~85 items |
| `ProductCliffordAlgebra<128>` | 1024 | ~147 items |

### Capacity Model

Holographic memory has capacity O(dim / log dim). As more items are stored, signal-to-noise ratio (SNR) degrades. Minuet provides tools to monitor and manage capacity.

---

## Module Reference

### traits Module

Core trait definitions that all Minuet types implement.

#### `MemoryTrace`

The fundamental storage unit—a holographic superposition of items.

```rust
pub trait MemoryTrace: Clone + Send + Sync {
    type Algebra: BindingAlgebra;

    // Properties
    fn dimension(&self) -> usize;
    fn item_count(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn theoretical_capacity(&self) -> usize;
    fn utilization(&self) -> f64;

    // Modification
    fn add(&mut self, item: &Self::Algebra, weight: f64);
    fn add_unit(&mut self, item: &Self::Algebra);
    fn merge(&mut self, other: &Self, weight: f64);
    fn clear(&mut self);

    // Query
    fn similarity(&self, query: &Self::Algebra) -> f64;
    fn unbind(&self, query: &Self::Algebra) -> Self::Algebra;
    fn as_algebra(&self) -> &Self::Algebra;

    // Diagnostics
    fn estimated_snr(&self) -> f64;
    fn near_capacity(&self, threshold: f64) -> bool;
}
```

#### `MemoryStore`

Higher-level storage managing one or more traces.

```rust
pub trait MemoryStore: Send + Sync {
    type Trace: MemoryTrace;
    type Algebra: BindingAlgebra;

    // Storage
    fn store(&self, key: &Self::Algebra, value: &Self::Algebra) -> MinuetResult<StoreReceipt>;
    fn store_with_options(&self, key: &Self::Algebra, value: &Self::Algebra, options: StoreOptions) -> MinuetResult<StoreReceipt>;
    fn store_batch(&self, pairs: &[(Self::Algebra, Self::Algebra)]) -> MinuetResult<Vec<StoreReceipt>>;

    // Retrieval
    fn retrieve(&self, key: &Self::Algebra) -> MinuetResult<RetrievalResult<Self::Algebra>>;

    // Management
    fn capacity_info(&self) -> CapacityInfo;
    fn clear(&self) -> MinuetResult<()>;
    fn trace_count(&self) -> usize;
    fn total_items(&self) -> usize;
}
```

#### `Retriever`

Cleanup strategy for raw retrieval results.

```rust
pub trait Retriever: Send + Sync {
    type Algebra: BindingAlgebra;

    fn cleanup(&self, raw: &Self::Algebra, context: &RetrievalContext<Self::Algebra>) -> MinuetResult<CleanupResult<Self::Algebra>>;
    fn cleanup_batch(&self, raws: &[Self::Algebra], context: &RetrievalContext<Self::Algebra>) -> MinuetResult<Vec<CleanupResult<Self::Algebra>>>;
}
```

#### `Codebook`

Symbol vocabulary with stable representations.

```rust
pub trait Codebook: Send + Sync {
    type Algebra: BindingAlgebra;

    fn symbol(&self, name: &str) -> Self::Algebra;
    fn get(&self, name: &str) -> Option<Self::Algebra>;
    fn contains(&self, name: &str) -> bool;
    fn register(&self, name: &str, repr: Self::Algebra) -> MinuetResult<()>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn all_symbols(&self) -> Vec<Self::Algebra>;
    fn all_names(&self) -> Vec<String>;
    fn closest(&self, repr: &Self::Algebra) -> Option<(String, f64)>;
}
```

#### `CapacityPolicy`

Policy for handling capacity pressure.

```rust
pub trait CapacityPolicy: Send + Sync {
    fn can_accept(&self, info: &CapacityInfo) -> bool;
    fn warning_threshold(&self) -> f64;    // Default: 0.8
    fn critical_threshold(&self) -> f64;   // Default: 0.95
}
```

#### Supporting Types

```rust
pub struct StoreOptions {
    pub partition: Option<String>,  // Target partition
    pub weight: f64,                // Item weight (default: 1.0)
    pub source_id: Option<u64>,     // For attribution
    pub force: bool,                // Skip capacity checks
}

pub struct StoreReceipt {
    pub id: u64,
    pub post_snr: f64,
    pub warning: Option<CapacityWarning>,
    pub location: String,
}

pub struct RetrievalResult<A> {
    pub value: A,
    pub confidence: f64,
    pub attribution: Vec<(u64, f64)>,
}

pub struct CapacityInfo {
    pub total_items: usize,
    pub theoretical_capacity: usize,
    pub utilization: f64,
    pub estimated_snr: f64,
    pub per_trace: Vec<TraceCapacityInfo>,
}

pub struct RetrievalContext<A> {
    pub codebook: Option<Vec<A>>,
    pub temperature: f64,           // Higher = harder selection
    pub max_iterations: usize,
    pub convergence_threshold: f64,
}

pub struct CleanupResult<A> {
    pub value: A,
    pub confidence: f64,
    pub iterations: usize,
    pub converged: bool,
    pub codebook_match: Option<usize>,
}
```

---

### store Module

Memory store implementations.

#### `DenseTrace<A>`

Standard dense trace representation.

```rust
use minuet::store::DenseTrace;

let mut trace = DenseTrace::<Algebra>::new();
trace.add(&key_value_binding, 1.0);

let similarity = trace.similarity(&query);
let raw_result = trace.unbind(&query);
```

**Methods:**
- `new()` → `DenseTrace<A>`: Create empty trace
- `with_capacity(cap: usize)` → `DenseTrace<A>`: Create with capacity hint
- All `MemoryTrace` trait methods

#### `SimpleStore<A>`

Single-trace store for simple use cases.

```rust
use minuet::store::SimpleStore;

let store = SimpleStore::<Algebra>::new();
store.store(&key, &value)?;
let result = store.retrieve(&key)?;
```

**Methods:**
- `new()` → `SimpleStore<A>`: Create with default capacity
- `with_capacity(cap: usize)` → `SimpleStore<A>`: Create with capacity hint
- All `MemoryStore` trait methods

#### `ShardedStore<A>`

Hash-sharded store for larger capacity. Keys are hashed to distribute across multiple traces.

```rust
use minuet::store::ShardedStore;

let store = ShardedStore::<Algebra>::with_shards(8);
store.store(&key, &value)?;
```

**Methods:**
- `new()` → `ShardedStore<A>`: Create with default shard count (4)
- `with_shards(n: usize)` → `ShardedStore<A>`: Create with n shards
- All `MemoryStore` trait methods

**Capacity scaling:** With N shards, effective capacity is N × single-trace capacity.

---

### encoding Module

Encoding infrastructure for symbol codebooks.

#### `HashMapCodebook<A>`

In-memory codebook with deterministic symbol generation.

```rust
use minuet::encoding::HashMapCodebook;

let codebook = HashMapCodebook::<Algebra>::new();

// Get or create symbol
let paris = codebook.symbol("paris");
let france = codebook.symbol("france");

// Same name always returns same representation
assert!(codebook.symbol("paris").similarity(&paris) > 0.99);

// Find closest match
if let Some((name, similarity)) = codebook.closest(&noisy_vector) {
    println!("Closest: {} (sim: {:.3})", name, similarity);
}
```

**Methods:**
- `new()` → `HashMapCodebook<A>`: Create empty codebook
- `with_symbols(iter)` → `HashMapCodebook<A>`: Create with pre-registered symbols
- All `Codebook` trait methods

---

### retrieval Module

Retrieval and cleanup strategies.

#### `DirectRetriever<A>`

Returns raw unbind results without cleanup.

```rust
use minuet::retrieval::DirectRetriever;

let retriever = DirectRetriever::<Algebra>::new();
let result = retriever.cleanup(&raw, &context)?;
// result.value == raw (no transformation)
```

#### `ResonatorRetriever<A>`

Iterative cleanup using resonator dynamics. Projects results toward codebook entries.

```rust
use minuet::retrieval::ResonatorRetriever;

let retriever = ResonatorRetriever::<Algebra>::new();

// With codebook for cleanup
let context = RetrievalContext::default()
    .with_codebook(codebook.all_symbols());

let result = retriever.cleanup(&raw, &context)?;
println!("Cleaned value (converged: {})", result.converged);
```

**Methods:**
- `new()` → `ResonatorRetriever<A>`: Create with default settings
- `with_max_iterations(n: usize)` → `ResonatorRetriever<A>`: Set max iterations
- `with_convergence_threshold(t: f64)` → `ResonatorRetriever<A>`: Set convergence threshold

---

### capacity Module

Capacity management policies.

#### `AcceptAllPolicy`

Always accepts new items (no capacity enforcement).

```rust
use minuet::capacity::AcceptAllPolicy;

let policy = AcceptAllPolicy;
assert!(policy.can_accept(&any_capacity_info));
```

#### `RejectPolicy`

Rejects new items above a utilization threshold.

```rust
use minuet::capacity::RejectPolicy;

let policy = RejectPolicy::with_threshold(0.9);
// Rejects when utilization > 90%
```

---

### pipeline Module

Fluent builders for composing memory systems.

#### `PipelineBuilder<A>`

Compose stores, retrievers, codebooks, and policies.

```rust
use minuet::pipeline::PipelineBuilder;
use minuet::capacity::RejectPolicy;

let pipeline = PipelineBuilder::<Algebra>::new()
    .with_store(ShardedStore::with_shards(4))
    .with_retriever(ResonatorRetriever::new())
    .with_codebook(HashMapCodebook::new())
    .with_capacity_policy(RejectPolicy::with_threshold(0.9))
    .build()?;

// Use the composed pipeline
let key = pipeline.symbol("key");
let value = pipeline.symbol("value");
pipeline.store(&key, &value)?;
```

**Builder Methods:**
- `new()` → `PipelineBuilder<A>`: Start building
- `with_store(store)` → `Self`: Set store implementation
- `with_retriever(retriever)` → `Self`: Set retrieval strategy
- `with_codebook(codebook)` → `Self`: Set symbol codebook
- `with_capacity_policy(policy)` → `Self`: Set capacity policy
- `build()` → `MinuetResult<Pipeline<A>>`: Build the pipeline

**Pipeline Methods:**
- `symbol(name: &str)` → `A`: Get symbol from codebook
- `store(key, value)` → `MinuetResult<StoreReceipt>`: Store association
- `retrieve(key)` → `MinuetResult<RetrievalResult<A>>`: Retrieve value
- `capacity_info()` → `CapacityInfo`: Get capacity status

---

### reference Module

Reference implementations for learning and simple use cases.

#### `SimpleMemory<A>`

Complete memory system with sensible defaults.

```rust
use minuet::reference::SimpleMemory;

let memory = SimpleMemory::<Algebra>::new();

// Store with string keys/values
memory.store_symbols("cat", "meow")?;
memory.store_symbols("dog", "bark")?;

// Recall
if let Some((value, confidence)) = memory.recall("cat")? {
    println!("{} (confidence: {:.3})", value, confidence);
}

// Capacity info
let info = memory.capacity_info();
println!("Utilization: {:.1}%", info.utilization * 100.0);
```

**Methods:**
- `new()` → `SimpleMemory<A>`: Create with defaults
- `store_symbols(key: &str, value: &str)` → `MinuetResult<()>`: Store string association
- `recall(key: &str)` → `MinuetResult<Option<(String, f64)>>`: Recall with closest match
- `symbol(name: &str)` → `A`: Get symbol from codebook
- `symbol_count()` → `usize`: Number of registered symbols
- `item_count()` → `usize`: Number of stored associations
- `capacity_info()` → `CapacityInfo`: Get capacity status

---

### optical Module (Feature-Gated)

Optical backend with checkpoint-based persistence. Enable with `features = ["optical"]`.

#### Overview

The optical module provides hardware abstraction for optical holographic computing systems (DMD + MMF). Key design principle: **hardware is a compute accelerator, not storage**. Logical state is persisted and regenerated on any hardware.

```
┌─────────────────────────────────────────────────────────┐
│                 CheckpointedOpticalMemory               │
│  • store() / retrieve() - optical hot paths             │
│  • checkpoint() - periodic persistence                  │
│  • restore() - hardware-independent recovery            │
├─────────────────────────────────────────────────────────┤
│     MemoryJournal     TMatrixFingerprint   OpticalHardware │
│      (portable)         (validation)         (backend)    │
└─────────────────────────────────────────────────────────┘
```

#### `SymbolicExpression`

Hardware-independent expression trees for memory content.

```rust
use minuet::optical::SymbolicExpression;

// Atomic symbols
let cat = SymbolicExpression::symbol("cat");

// Bindings (role-filler associations)
let agent_john = SymbolicExpression::role_filler("AGENT", "John");

// Nested bindings
let scene = SymbolicExpression::bind(
    SymbolicExpression::role_filler("AGENT", "John"),
    SymbolicExpression::role_filler("ACTION", "run"),
);

// Weighted bundles (superpositions)
let weighted = SymbolicExpression::bundle(vec![
    (1.0, SymbolicExpression::symbol("primary")),
    (0.5, SymbolicExpression::symbol("secondary")),
]);

// Expression analysis
println!("Nodes: {}, Depth: {}", scene.node_count(), scene.depth());
let symbols = scene.referenced_symbols();
```

**Variants:**
- `Symbol(SymbolId)`: Atomic symbol reference
- `Bind(Box<Expr>, Box<Expr>)`: Binding of two expressions
- `Bundle(Vec<(OrderedFloat<f32>, Expr)>)`: Weighted superposition

**Methods:**
- `symbol(name)` → `SymbolicExpression`: Create atomic symbol
- `bind(a, b)` → `SymbolicExpression`: Create binding
- `bundle(elements)` → `SymbolicExpression`: Create weighted bundle
- `bundle_uniform(elements)` → `SymbolicExpression`: Create uniform-weight bundle
- `role_filler(role, filler)` → `SymbolicExpression`: Convenience for bind(symbol(role), symbol(filler))
- `referenced_symbols()` → `Vec<&SymbolId>`: Get all referenced symbols
- `node_count()` → `usize`: Count nodes in expression tree
- `depth()` → `usize`: Maximum depth of expression tree
- `is_symbol()`, `is_bind()`, `is_bundle()` → `bool`: Type checks

#### `OpticalHardware` Trait

Abstraction over optical hardware (DMD + MMF + camera).

```rust
pub trait OpticalHardware: Send {
    fn id(&self) -> &str;
    fn dimensions(&self) -> (usize, usize);
    fn n_modes(&self) -> usize;
    fn temperature(&self) -> Result<f32, HardwareError>;
    fn display(&mut self, hologram: &BinaryHologram) -> Result<(), HardwareError>;
    fn measure(&mut self) -> Result<OpticalMeasurement, HardwareError>;
    fn quick_calibrate(&mut self) -> Result<HardwareCalibration, HardwareError>;
    fn full_calibrate(&mut self) -> Result<HardwareCalibration, HardwareError>;
    fn is_ready(&self) -> bool;
    fn reset(&mut self) -> Result<(), HardwareError>;
    fn diagnostics(&self) -> HashMap<String, String>;
}
```

#### `MockOpticalHardware`

Simulated hardware for testing.

```rust
use minuet::optical::MockOpticalHardware;

let mut hardware = MockOpticalHardware::new(42);  // Seed for determinism
println!("ID: {}", hardware.id());
println!("Dimensions: {:?}", hardware.dimensions());

// Simulate T-matrix drift (for testing fingerprint detection)
hardware.drift_t_matrix(0.1);
```

**Methods:**
- `new(seed: u64)` → `MockOpticalHardware`: Create with seed
- `with_config(seed, dimensions, n_modes)` → `MockOpticalHardware`: Custom config
- `drift_t_matrix(amount: f32)`: Simulate thermal/mechanical drift
- `set_temperature(temp: f32)`: Set simulated temperature
- `set_ready(ready: bool)`: Set ready state
- `seed()` → `u64`: Get creation seed

#### `TMatrixFingerprint`

Fast hardware validation without full calibration.

```rust
use minuet::optical::{TMatrixFingerprint, MockOpticalHardware};

let mut hardware = MockOpticalHardware::new(42);

// Capture fingerprint (5 probe patterns)
let fingerprint = TMatrixFingerprint::capture(&mut hardware, 5)?;

// Later: validate hardware state
match fingerprint.validate(&mut hardware)? {
    FingerprintValidation::Valid => println!("Hardware unchanged"),
    FingerprintValidation::Drifted { correlation, .. } => {
        println!("Drifted (correlation: {:.3})", correlation);
    }
    FingerprintValidation::DifferentHardware { expected_id, actual_id } => {
        println!("Different hardware: {} vs {}", expected_id, actual_id);
    }
    FingerprintValidation::NoFingerprint => println!("No baseline"),
}
```

**Thresholds:**
- Correlation > 0.95: Valid
- Correlation 0.70-0.95: Drifted
- Correlation < 0.70: Different hardware

**Fields:**
- `responses: Vec<ProbeResponse>`: Captured probe responses
- `hardware_id: String`: Hardware identifier
- `temperature_celsius: f32`: Temperature at capture
- `captured_at: u64`: Timestamp (ms since epoch)
- `n_modes: usize`: Number of optical modes

**Methods:**
- `capture(hardware, n_probes)` → `Result<TMatrixFingerprint>`: Capture fingerprint
- `validate(hardware)` → `Result<FingerprintValidation>`: Validate against current state

#### `MemoryJournal`

Append-only operation log with replay and compaction.

```rust
use minuet::optical::{MemoryJournal, MemoryOp, SymbolicExpression};

let mut journal = MemoryJournal::new(encoder_config, codebook_config);

// Append operations
journal.append(MemoryOp::store(
    SymbolicExpression::symbol("key"),
    SymbolicExpression::symbol("value"),
    1.0,
));

// Replay to get current state
let state = journal.replay_to_state();
println!("Associations: {}", state.associations.len());

// Compact (fold operations into base state)
journal.compact();

// Save/load
journal.save(&path)?;
let loaded = MemoryJournal::load(&path)?;
```

**Operation Types:**
- `Store { key, value, strength, timestamp }`: Store association
- `Strengthen { key, delta, timestamp }`: Increase strength
- `Decay { factor, timestamp }`: Apply global decay
- `Forget { key, timestamp }`: Remove association
- `RegisterSymbol { symbol, seed, timestamp }`: Register symbol

#### `CheckpointedOpticalMemory<H>`

Main optical memory system with persistence.

```rust
use minuet::optical::{
    CheckpointedOpticalMemory, CheckpointConfig,
    MockOpticalHardware, SymbolicExpression,
};
use amari_holographic::optical::{LeeEncoderConfig, CodebookConfig};
use std::time::Duration;
use std::path::PathBuf;

// Configuration
let config = CheckpointConfig {
    journal_path: PathBuf::from("/tmp/memory.bin"),
    interval: Duration::from_secs(300),  // 5 minutes
    max_ops_before_compact: 10_000,
};

// Create memory
let hardware = MockOpticalHardware::new(42);
let mut memory = CheckpointedOpticalMemory::new(
    hardware,
    encoder_config,
    codebook_config,
    config.clone(),
)?;

// Store associations
memory.store(
    SymbolicExpression::role_filler("AGENT", "John"),
    SymbolicExpression::role_filler("ACTION", "run"),
)?;

// Retrieve
if let Some(result) = memory.retrieve(&SymbolicExpression::role_filler("AGENT", "John"))? {
    println!("Value: {:?} (similarity: {:.3})", result.value, result.similarity);
}

// Checkpoint
memory.checkpoint()?;

// Later: restore (works on same or different hardware)
let new_hardware = MockOpticalHardware::new(999);  // Different hardware!
let restored = CheckpointedOpticalMemory::restore(new_hardware, config)?;
```

**Methods:**
- `new(hardware, encoder_config, codebook_config, checkpoint_config)` → `Result<Self>`
- `restore(hardware, checkpoint_config)` → `Result<Self>`: Restore from checkpoint
- `store(key, value)` → `Result<()>`: Store association
- `retrieve(key)` → `Result<Option<RetrievalResult>>`: Retrieve value
- `decay(factor)` → `Result<()>`: Apply decay to all associations
- `forget(key)` → `Result<()>`: Remove specific association
- `checkpoint()` → `Result<()>`: Save current state
- `stats()` → `MemoryStats`: Get memory statistics
- `hardware_info()` → `HardwareInfo`: Get hardware information

**Persistence Model:**
| Layer | Content | Portability |
|-------|---------|-------------|
| Semantic | Associations, relationships | Fully portable |
| Codebook | Symbol → seed mappings | Regenerable |
| Holograms | Binary patterns | Derived on demand |
| Calibration | T-matrix, learned patterns | Hardware-bound |

---

## Error Handling

All fallible operations return `MinuetResult<T>`, which is `Result<T, MinuetError>`.

```rust
use minuet::error::{MinuetError, MinuetResult};

fn example() -> MinuetResult<()> {
    // Operations that can fail
    memory.store(&key, &value)?;
    Ok(())
}
```

**Error Variants:**
- `MinuetError::Algebra(String)`: Algebra operation failed
- `MinuetError::Capacity(String)`: Capacity exceeded
- `MinuetError::Encoding(String)`: Encoding failed
- `MinuetError::Io(std::io::Error)`: I/O error (persistence)

---

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | Standard library support | - |
| `parallel` | Parallel operations | `rayon` |
| `serde` | Serialization | `serde`, `bincode` |
| `persistence` | RocksDB persistence | `rocksdb`, `serde` |
| `async` | Async support | `tokio` |
| `optical` | Optical backend | `serde`, `ordered-float`, `rand` |
| `full` | All features | All above |

Enable features in `Cargo.toml`:

```toml
[dependencies]
minuet = { version = "0.2", features = ["optical", "parallel"] }
```

---

## Examples

Run examples with:

```bash
# Basic examples (no features required)
cargo run --example simple_memory
cargo run --example compose_pipeline

# Optical examples (requires optical feature)
cargo run --example optical_memory_demo --features optical
cargo run --example optical_fingerprint_demo --features optical
cargo run --example optical_expressions_demo --features optical
```

---

## Version History

See [CHANGELOG.md](../CHANGELOG.md) for version history.

---

## License

MIT OR Apache-2.0
