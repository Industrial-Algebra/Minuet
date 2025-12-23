# Migration Guide: amari-fusion to amari-holographic

This document outlines the migration path from `amari-fusion::TropicalDualClifford` to `amari-holographic` algebras.

## Overview

The `amari-holographic` crate provides purpose-built algebras for holographic memory with:
- **Linear scaling**: O(K) instead of O(2^n) for dimension
- **Higher capacity**: Proper dimension-based capacity calculations
- **Multiple algebra options**: Choose the best fit for your use case
- **Clean API**: `BindingAlgebra` trait with fallible operations

## Quick Comparison

| Aspect | amari-fusion | amari-holographic |
|--------|--------------|-------------------|
| Primary type | `TropicalDualClifford<T, DIM>` | `ProductCliffordAlgebra<K>` |
| DIM meaning | Clifford generators (2^DIM basis) | Direct: K copies of Cl3 (8K dim) |
| 256-dim config | Not practical (DIM=8 → 256 basis) | `ProductCl3x32` (K=32) |
| 1024-dim config | Impossible (overflow) | `ProductCl3x128` (K=128) |
| Capacity (256D) | ~46 items | ~46 items |
| Compute (256D) | O(65,536) | O(2,048) |
| Trait | `Bindable` | `BindingAlgebra` |
| Inverse | `Option<Self>` | `AlgebraResult<Self>` |

## API Mapping

### Type Changes

```rust
// Before (amari-fusion)
use amari_fusion::{TropicalDualClifford, holographic::Bindable};
type Algebra = TropicalDualClifford<f64, 8>;  // 256 basis elements

// After (amari-holographic)
use amari_holographic::{ProductCliffordAlgebra, BindingAlgebra};
type Algebra = ProductCliffordAlgebra<32>;  // 256 dimensions (32 * 8)

// Or use type aliases
use amari_holographic::ProductCl3x32;  // Same as above
```

### Method Mapping

| amari-fusion | amari-holographic | Notes |
|--------------|-------------------|-------|
| `Bindable::bind(&other)` | `BindingAlgebra::bind(&other)` | Same signature |
| `Bindable::binding_inverse() -> Option<Self>` | `BindingAlgebra::inverse() -> AlgebraResult<Self>` | Error type change |
| `Bindable::unbind(&other)` | `BindingAlgebra::unbind(&other) -> AlgebraResult<Self>` | Now fallible |
| `Bindable::bundle(&other, beta)` | `BindingAlgebra::bundle(&other, beta) -> AlgebraResult<Self>` | Now fallible |
| `Bindable::similarity(&other) -> f64` | `BindingAlgebra::similarity(&other) -> f64` | Same |
| `Bindable::norm() -> f64` | `BindingAlgebra::norm() -> f64` | Same |
| `Bindable::normalize() -> Self` | `BindingAlgebra::normalize() -> AlgebraResult<Self>` | Now fallible |
| `Bindable::binding_identity()` | `BindingAlgebra::identity()` | Renamed |
| `Bindable::bundling_zero()` | `BindingAlgebra::zero()` | Renamed |
| `TropicalDualClifford::random()` | `ProductCliffordAlgebra::random_versor(2)` | Specify versor grade |
| `TropicalDualClifford::new()` | `ProductCliffordAlgebra::zero()` | Zero element |

### Error Handling

```rust
// Before: Option-based
let inv = x.binding_inverse();
match inv {
    Some(inverse) => { /* use inverse */ }
    None => { /* handle failure */ }
}

// After: Result-based
let inv = x.inverse();
match inv {
    Ok(inverse) => { /* use inverse */ }
    Err(AlgebraError::NotInvertible { reason }) => { /* handle */ }
    Err(e) => { /* other errors */ }
}

// Or with ? operator
let inv = x.inverse()?;
```

### Memory API

```rust
// Before (Minuet's MemoryTrace)
let mut memory = MemoryTrace::<f64, 8>::new();
memory.store(&key, &value)?;
let result = memory.retrieve(&key);

// After (amari-holographic's HolographicMemory)
let mut memory = HolographicMemory::<ProductCl3x32>::new(AlgebraConfig::default());
memory.store(&key, &value);  // Infallible
let result = memory.retrieve(&key);  // Returns RetrievalResult
```

### Capacity Info

```rust
// Before
let info = memory.capacity_info();
// info.theoretical_capacity was incorrectly DIM-based

// After
let info = memory.capacity_info();
// info.theoretical_capacity correctly uses algebra dimension
```

## Migration Steps

### Step 1: Update Cargo.toml

```toml
[dependencies]
# Remove or make optional:
# amari-fusion = { path = "..." }

# Add:
amari-holographic = { version = "0.12", features = ["parallel"] }

[features]
default = ["holographic"]
holographic = ["amari-holographic"]
legacy-fusion = ["amari-fusion"]  # Keep for transition
```

### Step 2: Create Algebra Abstraction

Use Minuet's `src/algebra.rs` abstraction layer to switch backends:

```rust
// With feature = "holographic" (default)
pub use amari_holographic::{
    ProductCliffordAlgebra as DefaultAlgebra,
    BindingAlgebra,
    AlgebraResult,
    AlgebraError,
};

pub type Algebra256 = amari_holographic::ProductCl3x32;
pub type Algebra1024 = amari_holographic::ProductCl3x128;
```

### Step 3: Update Type Aliases

```rust
// In src/lib.rs or prelude
pub type DefaultAlgebra = amari_holographic::ProductCl3x32;

// Or make it generic
pub struct MinuetConfig<A: BindingAlgebra> {
    // ...
}
```

### Step 4: Update Method Calls

Search and replace patterns:

```rust
// binding_inverse() -> inverse()
- x.binding_inverse()
+ x.inverse()

// Handle Result instead of Option
- if let Some(inv) = x.binding_inverse() { ... }
+ if let Ok(inv) = x.inverse() { ... }

// binding_identity() -> identity()
- TropicalDualClifford::binding_identity()
+ A::identity()

// bundling_zero() -> zero()
- TropicalDualClifford::bundling_zero()
+ A::zero()

// random() -> random_versor(2)
- TropicalDualClifford::random()
+ ProductCliffordAlgebra::random_versor(2)
```

### Step 5: Update Tests

```rust
// Before
let x: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

// After
let x: ProductCl3x32 = ProductCl3x32::random_versor(2);

// Capacity assertions
// Before (with our fix): expected ~32 items for DIM=8
// After: expected ~46 items for 256 dimensions
assert!(capacity > 40 && capacity < 55);
```

## Choosing an Algebra

| Use Case | Recommended Algebra | Why |
|----------|---------------------|-----|
| General holographic memory | `ProductCl3x32` to `ProductCl3x128` | Linear scaling, high capacity |
| Maximum capacity | `ProductCl3x256` (2048D) | ~280 items capacity |
| Simple operations | `FHRRAlgebra<256>` | Frequency domain, fast inverse |
| Hardware/embedded | `MAPAlgebra<256>` | Bipolar, self-inverse, XOR-friendly |
| Full geometric algebra | `CliffordAlgebra<8,0,0>` | When you need grade structure |

## Capacity Reference

| Configuration | Dimension | Capacity | SNR at 50% |
|---------------|-----------|----------|------------|
| ProductCl3x16 | 128 | ~26 | 2.2 |
| ProductCl3x32 | 256 | ~46 | 2.4 |
| ProductCl3x64 | 512 | ~85 | 2.5 |
| ProductCl3x128 | 1024 | ~147 | 2.6 |
| ProductCl3x256 | 2048 | ~280 | 2.7 |

## Breaking Changes Summary

1. **Trait rename**: `Bindable` → `BindingAlgebra`
2. **Error handling**: `Option<Self>` → `AlgebraResult<Self>` for inverse/unbind/bundle/normalize
3. **Identity/zero**: `binding_identity()` → `identity()`, `bundling_zero()` → `zero()`
4. **Random elements**: `random()` → `random_versor(num_factors)` or `random_unit()`
5. **Type parameter**: `<T, DIM>` → `<K>` (K = number of Cl3 factors)
6. **Dimension meaning**: DIM was generators (2^DIM basis), K is factors (8K dimension)

## Compatibility Period

During migration, you can support both backends:

```rust
#[cfg(feature = "holographic")]
mod algebra {
    pub use amari_holographic::*;
    pub type DefaultAlgebra = ProductCl3x32;
}

#[cfg(feature = "legacy-fusion")]
mod algebra {
    pub use amari_fusion::holographic::Bindable as BindingAlgebra;
    pub use amari_fusion::TropicalDualClifford;
    pub type DefaultAlgebra = TropicalDualClifford<f64, 8>;
}
```

## Files Requiring Migration

### Core Modules (High Priority)

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `src/algebra.rs` | Abstraction layer | Update with amari-holographic implementations |
| `src/lib.rs` | Public re-exports | Update `pub use amari_fusion::*` statements |
| `src/binding/algebra.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait instead |
| `src/binding/transform.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait instead |
| `src/binding/codebook.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait instead |

### Memory Module

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `src/memory/trace.rs` | `Bindable`, `TropicalDualClifford` | Replace with `Algebra` trait |
| `src/memory/query.rs` | `Bindable`, `TropicalDualClifford` | Replace with `Algebra` trait |
| `src/memory/store.rs` | `RetrievalResult`, `TropicalDualClifford` | Update return types |

### Retrieval Module

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `src/retrieval/resonator.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait |
| `src/retrieval/attribution.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait |

### Parallel Module

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `src/parallel/batch.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait |
| `src/parallel/sharded.rs` | Full `amari_fusion` import | Requires careful refactoring |
| `src/parallel/merge.rs` | `Bindable`, `TropicalDualClifford` | Use `Algebra` trait |

### Domain Encoders

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `src/domains/mod.rs` | `TropicalDualClifford` | Make generic over `Algebra` |
| `src/domains/symbolic.rs` | `Bindable`, `TropicalDualClifford` | Make generic over `Algebra` |
| `src/domains/geometric.rs` | `Bindable`, `TropicalDualClifford` | Make generic over `Algebra` |
| `src/domains/molecular.rs` | `Bindable`, `TropicalDualClifford` | Make generic over `Algebra` |

### Persistence Module

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `src/persistence/mod.rs` | `RetrievalResult`, `TropicalDualClifford` | Update serialization |
| `src/persistence/journal.rs` | `TropicalDualClifford` | Update serialization |
| `src/persistence/snapshot.rs` | `TropicalDualClifford` | Update serialization |

### Examples

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `examples/molecular_analogy.rs` | `Bindable` | Update to use `Algebra` |
| `examples/motor_primitives.rs` | `Bindable` | Update to use `Algebra` |
| `examples/working_memory_agent.rs` | `Bindable`, `TropicalDualClifford` | Full update needed |

### Tests and Benchmarks

| File | Usage | Migration Notes |
|------|-------|-----------------|
| `tests/algebraic_laws.rs` | `Bindable`, `TropicalDualClifford` | Update for new trait |
| `benches/retrieval_latency.rs` | `Bindable`, `TropicalDualClifford` | Update for benchmarking |
| `benches/capacity_scaling.rs` | `Bindable`, `TropicalDualClifford` | Update capacity tests |
| `benches/parallel_ops.rs` | `TropicalDualClifford` | Update for new types |
| `benches/binding_throughput.rs` | `Bindable`, `TropicalDualClifford` | Update for benchmarking |

## Migration Strategy

1. **Phase 1**: Use the `Algebra` trait abstraction in `src/algebra.rs`
2. **Phase 2**: When amari-holographic is released, add feature flags
3. **Phase 3**: Gradually update modules to use `Algebra` trait instead of direct imports
4. **Phase 4**: Test both backends with feature flags
5. **Phase 5**: Make amari-holographic the default, deprecate legacy-fusion
