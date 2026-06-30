# Getting Started with Holographic Memory

A zero-to-running introduction to Minuet. By the end you'll have stored and recalled
associations using holographic (vector-symbolic) memory, and know where to go next.

> **Prerequisite:** Rust nightly (Minuet uses nightly via `rust-toolchain.toml`; just
> `cargo build` and rustup picks the right toolchain).

## 1. Add Minuet

```toml
[dependencies]
minuet = "0.4"
amari-holographic = "0.23"
```

## 2. Your first holographic memory

The easiest entry point is `SimpleMemory` — a complete store + symbol vocabulary in one
type, generic over the binding algebra:

```rust
use minuet::prelude::*;
use minuet::reference::SimpleMemory;

// Pick an algebra. ProductCliffordAlgebra<K> has 8*K dimensions.
// K=8 → 64 dims (fast, small capacity); K=64 → 512 dims (~85 item capacity).
type Algebra = ProductCliffordAlgebra<64>;

fn main() -> MinuetResult<()> {
    let memory = SimpleMemory::<Algebra>::new();

    // Store associations between named symbols.
    memory.store_symbols("paris", "france")?;
    memory.store_symbols("berlin", "germany")?;

    // Recall: given a key, find the associated value.
    if let Some((value, confidence)) = memory.recall("paris")? {
        println!("paris -> {value} (confidence: {confidence:.2})");
    }
    Ok(())
}
```

### What just happened?

1. **Symbols become vectors.** Each name (`"paris"`, `"france"`, …) maps to a
   high-dimensional random vector in the algebra, managed by the codebook.
2. **Binding composes.** `store_symbols("paris", "france")` computes the geometric product
   `paris ⊛ france` — a new vector dissimilar to both, encoding the association.
3. **The trace is a superposition.** Both bindings are added into a single memory trace.
4. **Retrieval is algebraic.** `recall("paris")` *unbinds* (`paris⁻¹ ⊛ trace` ≈ `france`),
   then finds the closest named symbol by similarity.

## 3. Direct algebra operations

You're not limited to named symbols — you can bind and retrieve raw vectors:

```rust
use minuet::prelude::*;
use minuet::reference::SimpleMemory;

type Algebra = ProductCliffordAlgebra<64>;

fn main() -> MinuetResult<()> {
    let memory = SimpleMemory::<Algebra>::new();

    // Get the raw vector for a symbol.
    let dog = memory.symbol("dog");
    let bark = memory.symbol("bark");

    // Bind and store at the algebra level.
    memory.store(&dog, &bark)?;

    // Retrieve: the result is a vector, not a name.
    let result = memory.retrieve(&dog)?;
    // `result.value` is now ≈ `bark`. Find its nearest name:
    if let Some((name, confidence)) = memory.closest(&result.value) {
        println!("dog -> {name} (confidence: {confidence:.2})");
    }
    Ok(())
}
```

## 4. Composing a custom pipeline

For more control — sharded storage for capacity, resonator cleanup for noisy retrieval,
capacity policies — use the `PipelineBuilder`:

```rust
use minuet::prelude::*;
use minuet::capacity::RejectPolicy;
use minuet::pipeline::PipelineBuilder;
use minuet::store::ShardedStore;
use minuet::retrieval::ResonatorRetriever;
use minuet::encoding::HashMapCodebook;

type Algebra = ProductCliffordAlgebra<32>; // 256 dims

fn main() -> MinuetResult<()> {
    let pipeline = PipelineBuilder::<Algebra>::new()
        .with_store(ShardedStore::with_shards(4))         // 4× capacity via sharding
        .with_retriever(ResonatorRetriever::new())        // resonator cleanup
        .with_codebook(HashMapCodebook::new())            // symbol vocabulary
        .with_capacity_policy(RejectPolicy::with_threshold(0.9))
        .build()?;

    let key = pipeline.symbol("key");
    let value = pipeline.symbol("value");
    pipeline.store(&key, &value)?;

    let result = pipeline.retrieve(&key)?;
    println!("retrieved, confidence: {:.2}", result.confidence);
    Ok(())
}
```

## 5. Annealed cleanup (temperature)

When retrieval is noisy — the canonical case for a physical backend — annealed cleanup
starts soft (broad basin) and sharpens toward a hard (winner-take-all) selection across
iterations. Configure it from a `Temperature`:

```rust
use minuet::prelude::*;
use minuet::retrieval::{ResonatorRetriever, Temperature};
use minuet::traits::{RetrievalContext, Retriever};

type Algebra = ProductCliffordAlgebra<64>;

# fn main() -> MinuetResult<()> {
let symbols: Vec<Algebra> = (0..8).map(|_| Algebra::random_versor(2)).collect();

let retriever = ResonatorRetriever::new()
    .with_temperature(&Temperature::annealed(1.0, 100.0, 50)?); // soft → hard over 50 steps

let context = RetrievalContext::default().with_codebook(symbols.clone());
let result = retriever.cleanup(&symbols[3], &context)?;
assert!(result.converged);
# Ok(())
# }
```

See the `retrieval` module docs for `Temperature` (Soft/Hard/Beta/Annealed variants) and
`TemperatureSchedule` (linear/exponential/cosine schedules).

## 6. Attribution: *why* was this recalled?

`Attribution` quantifies how much each stored item contributed to a retrieval — useful
for introspection (why did the agent remember *that*?) and for the gradient-exact path:

```rust
use minuet::prelude::*;
use minuet::retrieval::Attribution;

type Algebra = ProductCliffordAlgebra<64>;

# fn main() -> MinuetResult<()> {
let mut attr = Attribution::<Algebra>::new();
let key = Algebra::random_versor(2);
let val = Algebra::random_versor(2);
attr.register(1, key.bind(&val));

// Against the retrieved superposition, gradient attribution sums to ~1.0.
let result = attr.compute_gradient(&key, &val)?;
println!("top contributor: {:?}", result.top_n(1));
# Ok(())
# }
```

`Attribution::compute` is a fast similarity-based heuristic; `Attribution::compute_gradient`
is the exact orthogonal projection (see its doc comment for the math and the sum-to-1
guarantee against the pure-sum superposition).

## Where to go next

| Topic | Where |
|-------|-------|
| Full API reference | `cargo doc --open` (or [docs.rs/minuet](https://docs.rs/minuet)) |
| Holographic memory concepts (binding, superposition, cleanup) | the crate-level docs in `src/lib.rs` |
| Optical / physical backend | the `optical` module + `docs/HANDOFF-kagome-readiness.md` |
| Performance numbers | `BENCHMARKS.md` |
| The Kagome-readiness sprint record | `docs/HANDOFF-kagome-readiness.md` |

## A note on recall confidence

You may notice recall `confidence` values that look low (e.g. ~0.2 for a handful of items).
This reflects Minuet's current bundling implementation (`DenseTrace::add` accumulates via a
softmax-weighted average, which dilutes earlier bindings). An additive superposition path
lands with `amari-holographic` 0.24.0 — see the [CHANGELOG](../CHANGELOG.md) and
[`Amari` PR #176](https://github.com/Industrial-Algebra/Amari/pull/176). The retrieved
*value* is correct regardless; only confidence is diluted.
