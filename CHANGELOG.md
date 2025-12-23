# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/industrial-algebra/minuet/releases/tag/v0.1.0
