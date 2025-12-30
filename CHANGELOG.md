# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.0]: https://github.com/industrial-algebra/minuet/releases/tag/v0.2.0
[0.1.0]: https://github.com/industrial-algebra/minuet/releases/tag/v0.1.0
