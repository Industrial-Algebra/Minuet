# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-06-22

The **Kagome-readiness sprint**: restore capability lost in the v0.3.0 tech-debt release,
close the amari version skew, and stand up the optical compute path the (forthcoming)
microwave back-end Kagome accelerates. See `docs/HANDOFF-kagome-readiness.md` for the
full sprint record.

### Added

- **Annealed temperature schedules** (`retrieval::temperature`): the `Temperature` enum
  (`Soft`/`Hard`/`Beta`/`Annealed`) and `TemperatureSchedule` (constant/linear/exponential/
  cosine) deleted in v0.3.0, restored near-verbatim and re-exported into the prelude.
  Drives resonator cleanup from a soft (broad basin) start toward a hard (tropical) finish
  across iterations — directly attacking the phase-noise error budget a noisy back-end
  faces. (WS 2, PR #11)
- **Attribution / provenance tracking** (`retrieval::Attribution<A>`): restored and
  re-expressed generically over `A: BindingAlgebra` (the pre-v0.3.0 code was hard-wired to
  `TropicalDualClifford`). `AttributionResult`, `AttributionQuery`, and the narrative
  types (`RetrievalExplanation`, `ExplanationFactor`, `FactorRelation`) ship with it.
  (WS 3, PR #13)
- **Real dual-number gradient attribution**: `Attribution::compute_gradient` is no longer a
  similarity-fallback stub — it computes the exact orthogonal projection
  `attribᵢ = ⟨rᵢ, result⟩ / ⟨result, result⟩` over the binding algebra, summing to ~1.0
  against the pure-sum superposition. Implemented on `amari-core` (no `amari-fusion`
  dependency). (WS 4b, PR #15)
- **Temperature → resonator wiring**: `ResonatorRetriever::with_temperature` and
  `with_temperature_schedule` configure cleanup annealing from the restored temperature
  types. `Temperature::to_resonator_config` is the translation seam. (WS 4, PR #14)
- **Optical compute path**: `CheckpointedOpticalMemory::optical_store` is no longer a no-op.
  It binds `key ⊛ value` and accumulates into an additive `memory_trace` superposition, with
  a new `measure_via_hardware()` for the display+measure round-trip. This is the compute a
  physical back-end (optical or Kagome's microwave resonators) accelerates. (WS 5, PR #16)
- **`[[test]]` discovery** for `tests/integration/end_to_end.rs` — it was nested one dir
  deep and silently not running in CI; now it runs. (WS 6, PR #18)
- `optical` standalone leg in the CI feature-combination matrix. (WS 6, PR #18)

### Changed

- **`amari-holographic` floor bumped `^0.15` → `^0.23`** (resolves `amari-holographic`
  0.23.0 + the transitive `amari-core` 0.23.0 upgrade). Zero source changes; the
  `amari-holographic` optical surface is byte-identical 0.15↔0.23, so the real payload is
  the `amari-core` upgrade (7 new modules) plus currency. (WS 1, PR #10)
- **`develop` re-synced from `main`** as the AGPL/conformance baseline; redundant stranded
  branches deleted. (WS 0)
- `DenseTrace::add` accumulates via `bundle(beta=1.0)` (unchanged) — a confirmed
  recall-quality regression for ≥3 items is documented and the affected integration test
  is `#[ignore]`d with an evidence note. The fix lives upstream: an additive `superpose`
  method on `BindingAlgebra`, targeting `amari-holographic` 0.24.0 (see
  `Amari/docs/plans/2026-06-21-additive-superpose-binding-algebra.md`). Minuet 0.4.x will
  adopt it once the amari floor moves.

### Fixed

- Corrected the Kagome-readiness handoff's central §3 premise (the amari bump does *not*
  "unlock" the optical-field algebra — those symbols were present at 0.15.1; the bump is for
  currency + the `amari-core` upgrade). (PR #9, PR #17)
- CI now runs with the integration tests actually discovered; pre-existing
  `unused-variable` warning in the integration test fixed so the test binary compiles under
  `RUSTFLAGS=-Dwarnings`. (WS 6, PR #18)

### Documentation

- Added `docs/HANDOFF-kagome-readiness.md` — the full sprint record (workstreams 0–6,
  locked decisions, verification checklist). (PR #9)
- README feature-flags table now documents the `persistence` feature's C++/RocksDB
  toolchain requirement (excluded from `full` and CI). (WS 6, PR #18)

### Notes

- GPU acceleration of the optical compute path is deferred to `amari-gpu` 0.25.0 (which
  integrates Borsalino as a feature gate upstream); Minuet takes no direct GPU dependency.
- The optical compute path is **store-side**: retrieval still resolves against the logical
  (symbolic) state. Wiring retrieval to prefer the optical trace is future work, relevant
  once real hardware (Kagome) accelerates it.
- Kagome's `--features minuet` build is verified against this release.

## [0.3.0] - 2026-05-26

### Added

- Integration tests: end-to-end pipeline workflow, sharded capacity distribution,
  capacity rejection flow, simple memory full workflow, codebook determinism
- Benchmarks: store throughput at varying load, retrieval latency, sharded scaling,
  binding/unbinding/similarity operations

### Changed

- **Relicensed** from MIT OR Apache-2.0 to **AGPL-3.0-only** with dual commercial licensing
- Added `rust-toolchain.toml` (nightly + rustfmt + clippy) per IA ecosystem standards
- Added SPDX license headers (`Copyright (C) 2026 Industrial Algebra`) to all source files
- Updated `Cargo.toml`: `license = "AGPL-3.0-only"`, `rust-version = "1.75"`,
  expanded description
- Removed `persistence` from `full` feature set (requires C++ build tools for RocksDB)
- Added IA conformance badge to README

### Fixed

- Fixed `DenseTrace::as_algebra()` — was `unimplemented!()`, now returns cloned trace
- Fixed clippy warning: `Duration::from_secs(300)` → `Duration::from_mins(5)`
- Fixed all 11 doc-tests — changed from `rust,ignore` to verified `rust` blocks
- `as_algebra()` trait signature changed from `&Self::Algebra` to `Self::Algebra`
  (enables proper implementation behind `RwLock`)

### Removed

- Removed dead code: `retrieval/resonator.rs`, `retrieval/attribution.rs`,
  `retrieval/temperature.rs` (old `amari_fusion` prototype code, not compiled)
- Removed dead benchmarks referencing `amari_fusion`
- Removed misplaced `docs/GHRR-implementation-plan.md` (belongs in amari-holographic)

### Added (Documentation)

- `CONTRIBUTING.md` — CLA requirements, dev setup, PR process
- `HANDOFF.md` — Agent hand-off document with architecture overview
- `docs/ROADMAP.md` — Future directions across near/medium/far-term horizons

### Test Coverage

- 41 unit tests (up from 28)
- 5 integration tests
- 11 verified doc-tests (up from 0)
- Total: **57 tests passing**

## [0.2.0] - 2024-12-29

### Added

- **Optical Backend Module** (`optical` feature)
  - Hardware abstraction for optical computing (DMD + MMF systems)
  - Checkpoint-based persistence that's portable across hardware
  - T-matrix fingerprinting for fast hardware state validation

- **Symbolic Expression Types**
  - `SymbolicExpression` - hardware-independent memory representation
  - `Symbol`, `Bind`, `Bundle` variants for compositional expressions
  - `OrderedFloat<f32>` - hashable float wrapper for bundle weights

- **Memory Journal**
  - `MemoryJournal` - append-only operation log
  - `MemoryOp` - store, strengthen, decay, forget, register operations
  - `CompactedMemoryState` - snapshot for efficient restore
  - Replay and compaction support

- **T-Matrix Fingerprinting**
  - `TMatrixFingerprint` - compact hardware characterization
  - `ProbePattern` / `ProbeResponse` - probe-based validation
  - `FingerprintValidation` - detect valid, drifted, or different hardware

- **Hardware Abstraction**
  - `OpticalHardware` trait - abstraction over real/simulated hardware
  - `OpticalMeasurement` - measurement result type
  - `HardwareCalibration` - calibration state with pattern cache
  - `HardwareError` - comprehensive error types

- **Mock Hardware**
  - `MockOpticalHardware` - simulated DMD + MMF for testing
  - Deterministic T-matrix generation from seed
  - T-matrix drift simulation for testing fingerprint detection

- **Checkpointed Optical Memory**
  - `CheckpointedOpticalMemory<H>` - main optical memory system
  - Hot path store/retrieve with minimal persistence overhead
  - Automatic checkpoint on configurable interval
  - Restore on same or different hardware

- **New Example**
  - `optical_memory_demo` - demonstrates optical backend with persistence

### Changed

- Version bumped to 0.2.0
- Uses local path for `amari-holographic` (optical module not yet on crates.io)

### Dependencies

- Added `ordered-float` 4.5 (with serde feature)
- Added `rand` 0.8 and `rand_chacha` 0.3 for mock hardware

## [0.1.0] - 2024-12-23

### Added

- **Core Traits**: Generic trait system for holographic memory components
  - `MemoryTrace` - fundamental storage unit for items in superposition
  - `MemoryStore` - higher-level storage with key-value operations
  - `Retriever` - cleanup strategies for noisy retrievals
  - `Encoder` - domain object encoding
  - `Codebook` - symbol vocabularies with stable representations
  - `CapacityPolicy` - capacity management strategies

- **Store Module**
  - `DenseTrace<A>` - dense trace representation
  - `SimpleStore<A>` - single-trace store for simple use cases
  - `ShardedStore<A>` - hash-sharded store for larger capacity (N shards = ~N× capacity)

- **Encoding Module**
  - `HashMapCodebook<A>` - in-memory symbol codebook with deterministic generation

- **Retrieval Module**
  - `DirectRetriever<A>` - return raw results without cleanup
  - `ResonatorRetriever<A>` - iterative cleanup via resonator network

- **Capacity Module**
  - `RejectPolicy` - refuse new items at capacity threshold
  - `AcceptAllPolicy` - always accept (no capacity management)

- **Pipeline Module**
  - `PipelineBuilder<A>` - fluent API for composing memory systems
  - `Pipeline<A>` - composed memory pipeline

- **Reference Implementations**
  - `SimpleMemory<A>` - minimal complete memory combining store + codebook

- **Examples**
  - `simple_memory` - basic store-and-recall operations
  - `compose_pipeline` - custom pipeline composition with sharding

- **Feature Flags**
  - `std` (default) - standard library support
  - `parallel` - rayon parallelism
  - `serde` - serialization support
  - `persistence` - RocksDB storage (requires `serde`)
  - `async` - tokio async support
  - `full` - all features

### Notes

- Built on [`amari-holographic`](https://crates.io/crates/amari-holographic) v0.12
- Requires Rust nightly (for future `amari-gpu` compatibility)
- Generic over any `BindingAlgebra` implementation

[0.4.0]: https://github.com/industrial-algebra/Minuet/releases/tag/v0.4.0
[0.3.0]: https://github.com/industrial-algebra/Minuet/releases/tag/v0.3.0
[0.2.0]: https://github.com/industrial-algebra/minuet/releases/tag/v0.2.0
[0.1.0]: https://github.com/industrial-algebra/minuet/releases/tag/v0.1.0
[Unreleased]: https://github.com/industrial-algebra/Minuet/compare/v0.4.0...HEAD
