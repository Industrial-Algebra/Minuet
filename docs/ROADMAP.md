# Minuet — Directions

**Version:** 0.3.0 — IA-conformant. Licensed. Cleaned up.
**Gitflow:** `main` (releases) ← `develop` (integration) ← `feature/*` (work)

---

## Current State (v0.3.0)

Minuet provides a holographic memory toolkit built on `amari-holographic`. It is
embeddable, synchronous, and generic over any `BindingAlgebra`. Features include
sharded storage, resonator-based retrieval, pipeline composition, and an optical
backend with checkpoint persistence for DMD/MMF hardware.

**Completed in v0.3.0:**
- ✅ IA ecosystem conformance (AGPL-3.0 dual-licensing, SPDX headers, toolchain)
- ✅ Dead code removal (3 files, 4 benchmarks, GHRR plan)
- ✅ Fixed all doc-tests (11 passing, up from 0)
- ✅ Fixed `as_algebra()` stub in DenseTrace
- ✅ Fixed clippy warning
- ✅ Expanded test coverage (41 unit + 5 integration + 11 doc = 57 total)
- ✅ New benchmarks (store, retrieve, sharding, binding)
- ✅ Persistence feature documented (requires C++ toolchain, excluded from `full`)
- ✅ CONTRIBUTING.md, HANDOFF.md, ROADMAP.md

---

## Near-Term (v0.4.0)

### 1. Complete Optical Store Hot Path
The `optical_store()` method in `CheckpointedOpticalMemory` is currently a stub.
Implement true optical superposition: bind key with value, bundle with existing
memory trace, display and measure for resonator cleanup.

### 2. Persistence Module
Implement a `persistence` module behind the existing feature gate using RocksDB
or an alternative backend. The journal-based checkpoint model works for the optical
backend; extend it to the core holographic stores.

### 3. Attribution Tracking
Implement provenance/attribution tracking for retrieval results. When a value is
retrieved, identify which stored items contributed and in what proportion. This
was partially designed in the old `attribution.rs` prototype (now removed).

### 4. Temperature Control
Refine bundling temperature control. Currently DenseTrace uses a fixed β=1.0. Add
soft/hard/annealed temperature schedules for improved retrieval accuracy at high
loads.

### 5. Eviction Policies
Beyond `RejectPolicy` and `AcceptAllPolicy`, implement:
- `EvictOldestPolicy` — Remove oldest associations
- `EvictWeakestPolicy` — Remove weakest (lowest strength) associations
- `ConsolidatePolicy` — Merge similar traces to recover capacity

---

## Medium-Term (Research-Adjacent)

### 6. Schubert Integration — Holographic Access Control
Integrate with Schubert's access control model. Capabilities become binding vectors
in Minuet's holographic representation. Access is granted when the query vector's
similarity to the capability vector exceeds the trust threshold. Schubert's
wall-crossing engine determines which memories are accessible at each trust level.

### 7. GHRR Algebra Support
When `amari-holographic` adds GHRR (Generalized Holographic Reduced Representations),
Minuet can leverage non-commutative binding for sequence/order encoding. This
enables temporal memory (order of events matters) and hierarchical composition.

### 8. Async Memory Operations
Expand the `async` feature beyond the current tokio dependency. Implement:
- `AsyncMemoryStore` trait
- Non-blocking operations for high-throughput systems
- Streaming batch operations

### 9. Sparse Trace Support
Current traces are dense (all coefficients stored). For very high-dimensional
algebras, sparse traces would dramatically reduce memory usage. Implement
`SparseTrace<A>` as an alternative to `DenseTrace<A>`.

### 10. GPU Acceleration
The optical module's hologram generation and field operations are embarrassingly
parallel. When `amari-gpu` ships, implement GPU-accelerated variants of:
- Lee encoder hologram generation
- Optical field binding/bundling
- T-matrix fingerprint capture

---

## Far-Term (Speculative)

### 11. Distributed Holographic Memory
Operadic composition over a distributed system using CRDTs. Keys hash-sharded
across nodes. Binding operations commute, enabling eventually-consistent
holographic memory.

### 12. Formal Verification
Karpal integration for compile-time verification of memory invariants:
- Capacity bounds are never violated
- Shard distribution is correct
- Retrieval preserves associational structure
- Checkpoint journal is replay-consistent

### 13. Neural-Symbolic Bridge
Combine holographic memory with neural networks. Use holographic traces as
differentiable memory for transformer architectures, enabling continuous
associative recall during training.

### 14. Holographic File System
A POSIX-like filesystem where paths are bindings and directory listings are
bundles. Content-addressable storage with holographic retrieval. "Find files
similar to this one" becomes a similarity query.

---

## Design Principles (Preserved Across All Directions)

1. **Generic over algebra.** Never hard-code to a specific algebra type. All
   functionality must work with any `BindingAlgebra`.

2. **Hot paths are zero-IO.** Store and retrieve must never block on disk or
   network. Persistence happens via asynchronous checkpoints.

3. **Hardware is a compute accelerator.** The optical backend treats hardware
   as an accelerator, not storage. State is persisted independently.

4. **Capacity degrades gracefully.** Never fail catastrophically. Always
   return the best available result with a confidence score.

5. **Synchronous by default.** The core API is synchronous. Async is an
   optional enhancement, not a requirement.

6. **Symbol-addressable.** All memory operations use symbols, not raw vectors.
   The codebook provides the stable mapping.

---

*Minuet v0.3.0 — May 2026*
