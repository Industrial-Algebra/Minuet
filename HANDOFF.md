# Minuet — Agent Hand-Off

**Project:** Holographic memory toolkit built on amari-holographic
**Location:** `/home/elliotthall/working/industrial-algebra/Minuet`
**Date:** May 2026 (relicensed to AGPL-3.0, IA-conformance update)
**Status:** v0.2.0 foundation complete. Relicensed. Tech debt partially resolved. Ready for v0.3.0.

---

## What Is Minuet?

> "The optical table for holographic computing."

A Rust library extending `amari-holographic` with higher-level abstractions for cognitive memory systems. Minuet provides:

- **Memory Stores**: Sharded, partitioned, layered memory configurations
- **Retrieval Strategies**: Direct, resonator-based cleanup
- **Encoding Infrastructure**: Symbol codebooks with deterministic generation
- **Capacity Management**: Monitoring, eviction policies, rejection thresholds
- **Pipeline Composition**: Fluent builders for assembling memory systems
- **Optical Backend** (feature-gated): Hardware abstraction for DMD + MMF systems with checkpoint persistence and T-matrix fingerprinting

Named after Star Trek's first sentient hologram — memory that participates in cognition rather than merely serving it.

---

## Current State

### Tests: 69 passing (optical feature), 0 failing
```
unit tests:     28 passed (default features)
optical tests:  41 passed (--features optical)
total:          69 passed
cliipy:         1 minor warning (Duration::from_secs → from_mins)
build:          ✅ cargo build, ✅ cargo build --examples
```

### Feature Gated
- `std` (default) — standard library support
- `parallel` — rayon batch operations
- `serde` — Serialize/Deserialize via serde + bincode
- `persistence` — RocksDB storage (⚠️ requires C++ toolchain, not tested in CI)
- `async` — tokio async support
- `optical` — Optical backend with checkpoint persistence
- `full` — all of the above

### Implemented

#### Core Holographic Memory
- `DenseTrace<A>` — Fundamental storage unit, items in superposition
- `SimpleStore<A>` — Single-trace store for simple use cases
- `ShardedStore<A>` — Hash-sharded across N traces for N× capacity
- `HashMapCodebook<A>` — Deterministic symbol→vector mapping
- `DirectRetriever<A>` — Pass-through (no cleanup)
- `ResonatorRetriever<A>` — Iterative cleanup via amari-holographic Resonator
- `RejectPolicy` / `AcceptAllPolicy` — Capacity management
- `PipelineBuilder<A>` / `Pipeline<A>` — Fluent composition
- `SimpleMemory<A>` — Reference implementation combining store + codebook

#### Optical Backend (`optical` feature)
- `OpticalHardware` trait — Abstraction over DMD/MMF systems
- `MockOpticalHardware` — Simulated hardware with deterministic T-matrix
- `SymbolicExpression` — Hardware-independent expression trees (Symbol/Bind/Bundle)
- `MemoryJournal` — Append-only operation log with replay and compaction
- `TMatrixFingerprint` — Fast hardware validation without full recalibration
- `CheckpointedOpticalMemory<H>` — Main optical memory with checkpoint persistence

#### Examples
- `simple_memory` — Basic store-and-recall with strings
- `compose_pipeline` — Custom pipeline with sharding and resonator retrieval
- `optical_memory_demo` — Full optical demo with checkpoint/restore on same and different hardware
- `optical_fingerprint_demo` — T-matrix validation, drift detection
- `optical_expressions_demo` — Symbolic expression trees, serialization

---

## Project Architecture

```
Minuet/
├── Cargo.toml              # Depends on: amari-holographic v0.15, thiserror, parking_lot, dashmap, tracing
├── rust-toolchain.toml     # Nightly channel, rustfmt + clippy (IA ecosystem standard)
├── LICENSE                 # AGPL v3 full text
├── LICENSE-COMMERCIAL      # Commercial licensing terms
├── CONTRIBUTING.md         # CLA requirements, dev setup, PR process
├── README.md               # Full project documentation
├── CHANGELOG.md            # v0.1.0 → v0.2.0
├── src/
│   ├── lib.rs              # Crate root, re-exports, prelude, dimension utils
│   ├── traits.rs           # Core trait definitions (MemoryTrace, MemoryStore, Retriever, etc.)
│   ├── error.rs            # MinuetError enum (15 variants), MinuetResult<T>
│   ├── store/
│   │   ├── trace.rs        # DenseTrace<A> — fundamental storage unit
│   │   ├── simple.rs       # SimpleStore<A> — single-trace store
│   │   └── sharded.rs      # ShardedStore<A> — hash-sharded store
│   ├── encoding/
│   │   └── codebook.rs     # HashMapCodebook<A> — symbol vocabulary
│   ├── retrieval/
│   │   ├── direct.rs       # DirectRetriever<A> — pass-through
│   │   └── resonator_retriever.rs  # ResonatorRetriever<A> — cleanup via resonator
│   ├── capacity/
│   │   └── mod.rs          # RejectPolicy, AcceptAllPolicy
│   ├── pipeline/
│   │   └── builder.rs      # PipelineBuilder<A> + Pipeline<A>
│   ├── reference/
│   │   └── simple_memory.rs # SimpleMemory<A> — reference implementation
│   └── optical/            # Optical backend (feature-gated)
│       ├── mod.rs          # Module exports
│       ├── symbolic.rs     # SymbolicExpression, OrderedFloat<f32>
│       ├── journal.rs      # MemoryJournal, MemoryOp, CompactedMemoryState
│       ├── fingerprint.rs  # TMatrixFingerprint, probe/response
│       ├── hardware.rs     # OpticalHardware trait, OpticalMeasurement
│       ├── mock_hardware.rs # MockOpticalHardware
│       ├── checkpoint.rs   # CheckpointedOpticalMemory<H>
│       └── tests.rs        # Optical integration tests (41 tests)
├── examples/
│   ├── simple_memory.rs
│   ├── compose_pipeline.rs
│   ├── optical_memory_demo.rs
│   ├── optical_fingerprint_demo.rs
│   └── optical_expressions_demo.rs
├── docs/
│   ├── API.md              # Complete API reference
│   └── ROADMAP.md          # Future directions
└── tests/
    └── integration/        # (populated in Phase 3)
```

### Dependency Boundary
Minuet depends on:
- `amari-holographic` v0.15 — Core binding algebras and holographic operations
- `thiserror` v2 — Error derive macro
- `tracing` v0.1 — Instrumentation
- `parking_lot` v0.12 — Efficient synchronization
- `dashmap` v6 — Concurrent hash maps
- `num-traits` v0.2 — Numeric trait bounds

Optional dependencies (behind feature gates):
- `rayon` v1.10 — Parallel operations
- `serde` v1 + `bincode` v1.3 — Serialization
- `ordered-float` v4.5 — Hashable floats (optical)
- `rand` v0.8 + `rand_chacha` v0.3 — RNG (optical)
- `rocksdb` v0.22 — Persistence (⚠️ C++ build)
- `tokio` v1 — Async runtime

---

## Key Design Decisions

### 1. Generic Over BindingAlgebra
All Minuet types are generic over `A: BindingAlgebra` from `amari-holographic`. Default choice is `ProductCliffordAlgebra<K>` with 8×K dimensions.

### 2. Capacity Scales as O(D/ln D)
Holographic memory degrades gracefully with item count. For 512 dimensions, ~85 items fit before SNR degrades below useful levels. Sharding multiplies capacity linearly.

### 3. Optical Hardware is a Compute Accelerator
The optical backend treats hardware as a compute accelerator, not storage. State is persisted via checkpoints and regenerated on any hardware. This is the key design insight: persistence portability.

### 4. Persistence Model (4 layers)
| Layer | Content | Portability |
|-------|---------|-------------|
| Semantic | Associations, relationships | Fully portable |
| Codebook | Symbol → seed mappings | Regenerable |
| Holograms | Binary patterns | Derived on demand |
| Calibration | T-matrix, learned patterns | Hardware-bound |

### 5. Hot Path Design
`store()` and `retrieve()` are hot paths with zero I/O. Persistence happens via periodic checkpoints only.

---

## Integration Points with Schubert

Schubert Roadmap item 14: "Access Control for Holographic Memory." When implemented:
- Schubert capabilities become binding vectors in Minuet's holographic representation
- Access is granted when query vector similarity exceeds trust threshold
- The wall-crossing engine determines which memories are accessible at each trust level
- Minuet's `optical` module provides the hardware backend for optical access control

---

## Technical Debt & Known Issues

1. **Persistence feature** (`rocksdb`) requires C++ toolchain — not tested in CI
2. **1 clippy warning**: `Duration::from_secs(300)` should use `from_mins(5)` in checkpoint.rs
3. **`as_algebra()` stub**: `DenseTrace::as_algebra()` calls `unimplemented!()`
4. **`optical_store()` stub**: Hot path is a no-op; retrieval uses iterated field similarity
5. **Dead benchmark files**: 4 files in `benches/` use `amari_fusion` (not a dependency)
6. **Doc-tests**: 19 doc-tests use `rust,ignore` — not verified
7. **Integration tests**: `tests/integration/` directory is empty
8. **Limited test coverage**: Pipeline (2 tests) and capacity (3 tests) have minimal coverage

---

## Build & Test

```bash
cd /home/elliotthall/working/industrial-algebra/Minuet

# Build (zero warnings except 1 clippy note)
cargo build

# Run all tests
cargo test

# Run with optical backend
cargo test --features optical

# Run examples
cargo run --example simple_memory
cargo run --example compose_pipeline
cargo run --example optical_memory_demo --features optical

# Clippy (exclude persistence)
cargo clippy --features "parallel,serde,async,optical"

# Check docs
cargo doc --no-deps --all-features
```

---

## Conventions

- **Rust edition 2021**, nightly toolchain (IA ecosystem standard via `rust-toolchain.toml`)
- **`#![warn(missing_docs)]`** + **`#![warn(clippy::all)]`** + **`#![warn(clippy::pedantic)]`**
- **Feature gates are additive** — never break existing API
- **Error types via `thiserror`** — `MinuetError` has 15 structured variants
- **Synchronization via `parking_lot`** — `RwLock`, not `std::sync::RwLock`
- **Concurrent maps via `dashmap`** — used in ShardedStore
- **Copyright header on every source file** — SPDX `AGPL-3.0-only`
- **`#![allow(clippy::cast_precision_loss)]`** — intentional in numeric dimension code

---

## Quick Recipes

### Adding a New Store Implementation
1. Create `src/store/new_store.rs`
2. Implement `MemoryStore` trait (with `type Trace = DenseTrace<A>`)
3. Add `mod new_store;` to `src/store/mod.rs` and `pub use`
4. Add to `src/lib.rs` prelude
5. Add tests demonstrating store/retrieve/capacity

### Adding a New Algebra Backend
1. No code changes needed! All types are generic over `A: BindingAlgebra`
2. Just instantiate with the new algebra type: `SimpleMemory::<NewAlgebra>::new()`
3. Add capacity numbers to README table

### Adding a New Optical Hardware Implementation
1. Implement `OpticalHardware` trait for your hardware type
2. Use with `CheckpointedOpticalMemory::new(hardware, ...)`
3. Add fingerprinting tests with `TMatrixFingerprint::capture()`

---

## Licensing

Minuet is dual-licensed under:

- **GNU Affero General Public License v3 (AGPL-3.0-only)** — see `LICENSE`
- **Commercial License** — see `LICENSE-COMMERCIAL`

SPDX-License-Identifier: AGPL-3.0-only
Copyright (C) 2024-2026 Industrial Algebra. All rights reserved.

### License Headers
All source files carry SPDX headers:
```rust
// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
```

### Contributor License Agreement
Contributors must sign a CLA before pull requests can be merged.
See `CONTRIBUTING.md` for details.

---

## Amari Ecosystem Context

Minuet lives in the Industrial Algebra ecosystem at `/home/elliotthall/working/industrial-algebra/`:

| Project | What It Provides | Minuet Relevance |
|---------|-----------------|-----------------|
| **amari-holographic** v0.15 | BindingAlgebra, algebras, Resonator, optical primitives | Direct dependency |
| **amari** (23 crates) | Core math library | Underlying algebra implementations |
| **Schubert** | Access control via Schubert calculus | Future holographic access control integration |
| **Karpal** | Proof-carrying capabilities | Future formal verification target |

---

*Hand-off prepared May 2026. All tests passing (69 total). 1 clippy warning. Ready for v0.3.0 development.*
