# Minuet Benchmarks

Performance measurements for Minuet's holographic memory operations, captured via
[`criterion`](https://crates.io/crates/criterion). Run the benchmarks locally with:

```bash
cargo bench --features "parallel,serde,async,optical"
```

HTML reports land in `target/criterion/report/index.html`.

## Reference platform

| | |
|---|---|
| **CPU** | Intel Core Ultra 9 275HX |
| **Memory** | 61 GB |
| **Arch** | x86_64 |
| **Toolchain** | nightly (per `rust-toolchain.toml`) |

> Numbers are mean point estimates from criterion's 100-sample collection. They are
> representative of order-of-magnitude and scaling shape, not absolute claims — run
> locally for your hardware. The algebra is `ProductCliffordAlgebra<8>` (64 dimensions)
> unless noted.

## Store / retrieve (single-threaded)

`SimpleStore` over `ProductCliffordAlgebra<8>` at varying pre-load (items already in the
trace before the timed op).

| Operation | Pre-load | Time |
|-----------|---------:|-----:|
| `store` | 0 items | ~5.0 µs |
| `store` | 10 items | ~4.9 µs |
| `store` | 50 items | ~5.4 µs |
| `retrieve` | 10 items | ~650 ns |
| `retrieve` | 50 items | ~645 ns |

**Observations.** Store cost is roughly flat with load (the trace is a single bundled
element; `add` is one `bundle` op regardless of item count). Retrieve is ~7–8× faster than
store — it's an `unbind` + similarity, no write-lock acquisition or capacity computation.

## Sharded scaling

`ShardedStore` store throughput vs shard count (items distributed round-robin).

| Shards | Time per store |
|-------:|---------------:|
| 1 | ~5.9 µs |
| 4 | ~5.3 µs |
| 8 | ~5.2 µs |

**Observation.** Per-op latency converges quickly (4 shards ≈ 8 shards) because the work
per op is dominated by the binding arithmetic, not lock contention. Sharding's value is
*capacity* (each shard is an independent trace) and *concurrency* under multi-threaded
load — see `tests/stress/stress.rs::concurrent_read_write_sharded_store` for the
contention behavior, which this single-threaded benchmark does not exercise.

## Binding algebra primitives

Raw `BindingAlgebra` ops on `ProductCliffordAlgebra<8>`.

| Operation | Time |
|-----------|-----:|
| `bind` | ~510 ns |
| `unbind` | ~760 ns |
| `similarity` | ~230 ns |

**Observation.** `similarity` is the cheapest (no allocation, pure dot-product structure).
`unbind` is ~1.5× `bind` because it computes an inverse first (`reverse / |v|²`) then
binds. These three primitives dominate store/retrieve latency, which is why retrieve
(~650 ns ≈ unbind + similarity) tracks the primitive sum closely.

## Reproducing

```bash
# Full bench suite (writes target/criterion/)
cargo bench --features "parallel,serde,async,optical"

# A single bench group
cargo bench --features "parallel,serde,async,optical" -- binding
```

To compare against a baseline, criterion reads prior runs from `target/criterion/` and
reports % change; commit that directory (or a subset) to establish a regression baseline.

## What is *not* benchmarked yet

- **Optical mock vs in-memory** — the `CheckpointedOpticalMemory` / `MockOpticalHardware`
  round-trip (`measure_via_hardware`) is exercised by unit/stress tests but not yet a
  criterion benchmark. It would isolate the encoder + hardware-simulation overhead.
- **Multi-threaded store throughput** — the sharded bench above is single-threaded; a
  rayon-parallel or thread::scope benchmark would show the sharding contention payoff.
- **Algebra dimension sweep** — all numbers above use `PCA<8>` (64-dim). A `PCA<16/32/64>`
  sweep would show the dimension-cost trade-off (the README's capacity table lists the
  theoretical capacities; matching them with latency is a natural follow-up).

These are tracked as benchmark extensions; file an issue if you need them.
