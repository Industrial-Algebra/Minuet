# GHRR Implementation Plan for Amari

**Target Crate:** `amari-holographic`
**Feature Name:** `ghrr`
**Priority:** Medium-High
**Estimated Effort:** 2-3 weeks

## References

- **Primary Paper:** [Generalized Holographic Reduced Representations](https://arxiv.org/abs/2405.09689) (Yeung, Zou, Imani - UC Irvine, 2024)
- **HTML Version:** [arxiv.org/html/2405.09689v1](https://arxiv.org/html/2405.09689v1)
- **Background:** [Tensor Products and Hyperdimensional Computing](https://arxiv.org/abs/2305.10572) (Qiu, 2023) - Reserved for Kakekotoba

---

## 1. Overview

### What is GHRR?

Generalized Holographic Reduced Representations extend FHRR by replacing scalar complex phases (U(1) group elements) with unitary matrices (U(m) group elements). This provides:

1. **Non-commutative binding** - Essential for sequence/order encoding
2. **Enhanced capacity** - Better storage of compositional structures
3. **Tunable expressiveness** - Parameter m controls commutativity vs. efficiency tradeoff

### Mathematical Definition

**Hypervector structure:**
```
H = [U₁, U₂, ..., U_D]ᵀ  where Uⱼ ∈ U(m) (m×m unitary matrices)
```

**Total dimensionality:** D × m² complex numbers

**Base vector construction (Proposition 4.1):**
```
Hⱼ = Qⱼ · Λⱼ

where:
  Qⱼ ∈ O(m) - orthogonal matrix (rotation/reflection)
  Λⱼ = diag(e^(iφ₁), ..., e^(iφₘ)) - diagonal phase matrix
```

### Operations

| Operation | Definition | Complexity |
|-----------|------------|------------|
| **Bind** | H₁ ⊗ H₂ = [U₁ⱼ · U₂ⱼ]ⱼ | O(D · m³) |
| **Unbind** | H₁ ⊘ H₂ = [U₁ⱼ · U₂ⱼ†]ⱼ | O(D · m³) |
| **Bundle** | H₁ ⊕ H₂ = normalize([U₁ⱼ + U₂ⱼ]ⱼ) | O(D · m²) |
| **Similarity** | δ(H₁,H₂) = (1/mD)·ℜ(tr(Σⱼ U₁ⱼ·U₂ⱼ†)) | O(D · m²) |

### Relationship to Existing Algebras

```
m=1:  GHRR<D,1> ≅ FHRR<D>        (commutative, scalar phases)
m=D:  GHRR<D,D> → Tensor Product  (exact, exponential space)
m=2-4: Sweet spot for most applications
```

---

## 2. API Design

### Type Definition

```rust
use num_complex::Complex64;

/// Generalized Holographic Reduced Representation
///
/// A hypervector of D components, each an m×m unitary matrix.
/// Total dimensionality: D × m² complex numbers.
///
/// # Type Parameters
/// - `D`: Number of components (hypervector dimension)
/// - `M`: Unitary matrix size (1 = standard FHRR)
///
/// # Properties
/// - M=1: Commutative binding (equivalent to FHRR)
/// - M>1: Non-commutative binding (order-sensitive)
/// - Capacity scales with D/ln(D), improved for bound vectors
///
/// # Example
/// ```rust
/// use amari_holographic::GHRRAlgebra;
///
/// // 512-dimensional with 2×2 unitary matrices
/// type Algebra = GHRRAlgebra<512, 2>;
///
/// let a = Algebra::random();
/// let b = Algebra::random();
///
/// // Non-commutative: a.bind(&b) ≠ b.bind(&a)
/// let ab = a.bind(&b);
/// let ba = b.bind(&a);
/// assert!(ab.similarity(&ba).abs() < 0.1);
/// ```
#[derive(Clone, Debug)]
pub struct GHRRAlgebra<const D: usize, const M: usize> {
    /// D components, each an M×M unitary matrix stored in row-major order
    components: [[Complex64; M * M]; D],
}
```

### Core Trait Implementation

```rust
impl<const D: usize, const M: usize> BindingAlgebra for GHRRAlgebra<D, M> {
    fn dimension(&self) -> usize {
        D * M * M * 2  // Real dimensionality (complex = 2 real)
    }

    fn bind(&self, other: &Self) -> Self {
        // Element-wise matrix multiplication: Uⱼ · Vⱼ
        Self {
            components: std::array::from_fn(|j| {
                matrix_multiply(&self.components[j], &other.components[j])
            }),
        }
    }

    fn unbind(&self, other: &Self) -> Self {
        // Element-wise multiplication by conjugate transpose: Uⱼ · Vⱼ†
        Self {
            components: std::array::from_fn(|j| {
                matrix_multiply(&self.components[j], &conjugate_transpose(&other.components[j]))
            }),
        }
    }

    fn bundle(&self, other: &Self, beta: f64) -> Result<Self, AlgebraError> {
        // Weighted sum with polar decomposition to maintain unitarity
        let result = Self {
            components: std::array::from_fn(|j| {
                let sum = matrix_add(
                    &matrix_scale(&self.components[j], 1.0 - beta),
                    &matrix_scale(&other.components[j], beta),
                );
                polar_decomposition_unitary(&sum)
            }),
        };
        Ok(result)
    }

    fn similarity(&self, other: &Self) -> f64 {
        // δ(H₁,H₂) = (1/mD) · ℜ(tr(Σⱼ Uⱼ · Vⱼ†))
        let trace_sum: Complex64 = (0..D)
            .map(|j| {
                let product = matrix_multiply(
                    &self.components[j],
                    &conjugate_transpose(&other.components[j]),
                );
                matrix_trace(&product)
            })
            .sum();

        trace_sum.re / (M * D) as f64
    }

    fn identity() -> Self {
        // Identity matrix for each component
        Self {
            components: [identity_matrix::<M>(); D],
        }
    }

    fn inverse(&self) -> Result<Self, AlgebraError> {
        // Conjugate transpose of each unitary matrix
        Ok(Self {
            components: std::array::from_fn(|j| {
                conjugate_transpose(&self.components[j])
            }),
        })
    }

    fn normalize(&self) -> Result<Self, AlgebraError> {
        // Polar decomposition to project back to unitary
        Ok(Self {
            components: std::array::from_fn(|j| {
                polar_decomposition_unitary(&self.components[j])
            }),
        })
    }
}
```

### Random Generation (Proposition 4.1)

```rust
impl<const D: usize, const M: usize> GHRRAlgebra<D, M> {
    /// Generate random hypervector with quasi-orthogonality guarantees.
    ///
    /// Uses the Q·Λ decomposition from Proposition 4.1:
    /// - Q: Random orthogonal matrix (from Haar measure)
    /// - Λ: Diagonal matrix of random phases
    pub fn random() -> Self {
        Self::random_with_seed(rand::random())
    }

    pub fn random_with_seed(seed: u64) -> Self {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);

        Self {
            components: std::array::from_fn(|_| {
                let q = random_orthogonal_matrix::<M>(&mut rng);
                let lambda = random_diagonal_phases::<M>(&mut rng);
                matrix_multiply(&q, &lambda)
            }),
        }
    }

    /// Generate with controlled non-commutativity.
    ///
    /// `diagonality` ∈ [0, 1]:
    /// - 0: Fully non-commutative (random Q)
    /// - 1: Commutative (Q = I, equivalent to FHRR)
    pub fn random_with_diagonality(diagonality: f64, seed: u64) -> Self {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);

        Self {
            components: std::array::from_fn(|_| {
                let q = interpolate_to_identity(
                    &random_orthogonal_matrix::<M>(&mut rng),
                    diagonality,
                );
                let lambda = random_diagonal_phases::<M>(&mut rng);
                matrix_multiply(&q, &lambda)
            }),
        }
    }
}
```

### Convenience Type Aliases

```rust
/// Standard GHRR configurations
pub type GHRR256x2 = GHRRAlgebra<256, 2>;   // 1024 complex dims, non-commutative
pub type GHRR512x2 = GHRRAlgebra<512, 2>;   // 2048 complex dims, non-commutative
pub type GHRR256x4 = GHRRAlgebra<256, 4>;   // 4096 complex dims, highly non-comm.

/// Backward compatibility: M=1 is FHRR
pub type GHRRScalar<const D: usize> = GHRRAlgebra<D, 1>;
```

---

## 3. Implementation Phases

### Phase 1: Core Matrix Operations (Week 1)

**Goal:** Implement efficient m×m unitary matrix operations.

**Tasks:**
- [ ] `matrix_multiply<M>` - O(m³) matrix multiplication
- [ ] `conjugate_transpose<M>` - O(m²) Hermitian adjoint
- [ ] `matrix_trace<M>` - O(m) trace computation
- [ ] `matrix_add<M>`, `matrix_scale<M>` - Basic arithmetic
- [ ] `identity_matrix<M>` - Identity generation
- [ ] `polar_decomposition_unitary<M>` - Project to nearest unitary

**Optimization notes:**
- For M=2: Use explicit 2×2 formulas (fastest)
- For M=3,4: Unroll loops
- For M>4: Consider BLAS/LAPACK via `ndarray` or `nalgebra`

**Tests:**
- Verify U · U† = I for random unitaries
- Verify polar decomposition produces valid unitaries
- Benchmark against naive implementations

### Phase 2: GHRR Algebra Implementation (Week 1-2)

**Goal:** Implement `BindingAlgebra` trait for `GHRRAlgebra<D, M>`.

**Tasks:**
- [ ] Struct definition with const generics
- [ ] `bind()` - Element-wise matrix multiplication
- [ ] `unbind()` - Element-wise multiplication by adjoint
- [ ] `bundle()` - Weighted sum + polar decomposition
- [ ] `similarity()` - Normalized trace inner product
- [ ] `identity()`, `inverse()`, `normalize()`

**Tests:**
- Identity laws: `x.bind(identity) == x`
- Inverse laws: `x.bind(x.inverse()) ≈ identity`
- Unbind recovers: `(a.bind(b)).unbind(a) ≈ b`
- Similarity bounds: `|similarity| ≤ 1`

### Phase 3: Random Generation (Week 2)

**Goal:** Implement quasi-orthogonal random vector generation per Proposition 4.1.

**Tasks:**
- [ ] `random_orthogonal_matrix<M>` - Haar-distributed O(m) matrices
- [ ] `random_diagonal_phases<M>` - Uniform phase angles
- [ ] `GHRRAlgebra::random()` and `random_with_seed()`
- [ ] `random_with_diagonality()` for controlled commutativity

**Algorithm for random orthogonal matrix:**
```rust
fn random_orthogonal_matrix<const M: usize>(rng: &mut impl Rng) -> [Complex64; M*M] {
    // Method: QR decomposition of random Gaussian matrix
    let gaussian: [[f64; M]; M] = std::array::from_fn(|_|
        std::array::from_fn(|_| rng.gen::<f64>() * 2.0 - 1.0)
    );
    qr_decomposition_q(&gaussian)
}
```

**Tests:**
- Quasi-orthogonality: random vectors have low similarity
- Reproducibility: same seed → same vector
- Distribution: similarity histogram matches expected

### Phase 4: Serialization & Features (Week 2-3)

**Goal:** Feature flags and serialization support.

**Tasks:**
- [ ] `serialize` feature with serde support
- [ ] `parallel` feature for parallel bundle/batch operations
- [ ] Efficient binary serialization (components only)
- [ ] JSON serialization for debugging

**Cargo.toml additions:**
```toml
[features]
ghrr = []  # Enable GHRR algebra
ghrr-blas = ["ghrr", "ndarray", "ndarray-linalg"]  # BLAS acceleration for large M
```

### Phase 5: Documentation & Examples (Week 3)

**Goal:** Comprehensive documentation and examples.

**Tasks:**
- [ ] Rustdoc for all public types/methods
- [ ] Example: Sequence encoding with non-commutative binding
- [ ] Example: Comparison of GHRR vs FHRR for ordered data
- [ ] Benchmark: Capacity comparison across M values
- [ ] Add to algebra comparison table in README

---

## 4. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    type Algebra = GHRRAlgebra<64, 2>;

    #[test]
    fn test_non_commutativity() {
        let a = Algebra::random_with_seed(1);
        let b = Algebra::random_with_seed(2);

        let ab = a.bind(&b);
        let ba = b.bind(&a);

        // Should be dissimilar for M > 1
        assert!(ab.similarity(&ba).abs() < 0.3);
    }

    #[test]
    fn test_commutativity_at_m1() {
        type Commutative = GHRRAlgebra<64, 1>;

        let a = Commutative::random_with_seed(1);
        let b = Commutative::random_with_seed(2);

        let ab = a.bind(&b);
        let ba = b.bind(&a);

        // Should be identical for M = 1 (FHRR)
        assert!(ab.similarity(&ba) > 0.99);
    }

    #[test]
    fn test_unbind_recovery() {
        let a = Algebra::random_with_seed(1);
        let b = Algebra::random_with_seed(2);

        let bound = a.bind(&b);
        let recovered = bound.unbind(&a);

        assert!(recovered.similarity(&b) > 0.95);
    }

    #[test]
    fn test_sequence_encoding() {
        let role = Algebra::random_with_seed(100);
        let dog = Algebra::random_with_seed(1);
        let bites = Algebra::random_with_seed(2);
        let man = Algebra::random_with_seed(3);

        // "dog bites man" vs "man bites dog"
        let s1 = role.bind(&dog).bind(&bites).bind(&man);
        let s2 = role.bind(&man).bind(&bites).bind(&dog);

        // Should be distinguishable
        assert!(s1.similarity(&s2).abs() < 0.5);
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_similarity_bounded(seed1: u64, seed2: u64) {
        let a = Algebra::random_with_seed(seed1);
        let b = Algebra::random_with_seed(seed2);

        let sim = a.similarity(&b);
        prop_assert!(sim >= -1.0 && sim <= 1.0);
    }

    #[test]
    fn prop_self_similarity_is_one(seed: u64) {
        let a = Algebra::random_with_seed(seed);
        prop_assert!((a.similarity(&a) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn prop_unitarity_preserved(seed: u64) {
        let a = Algebra::random_with_seed(seed);
        let inv = a.inverse().unwrap();
        let product = a.bind(&inv);

        prop_assert!(product.similarity(&Algebra::identity()) > 0.99);
    }
}
```

### Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_bind(c: &mut Criterion) {
    let mut group = c.benchmark_group("ghrr_bind");

    for m in [1, 2, 4].iter() {
        group.bench_with_input(BenchmarkId::new("M", m), m, |b, &m| {
            match m {
                1 => bench_bind_impl::<64, 1>(b),
                2 => bench_bind_impl::<64, 2>(b),
                4 => bench_bind_impl::<64, 4>(b),
                _ => unreachable!(),
            }
        });
    }
    group.finish();
}

fn bench_bind_impl<const D: usize, const M: usize>(b: &mut criterion::Bencher) {
    let a = GHRRAlgebra::<D, M>::random();
    let c = GHRRAlgebra::<D, M>::random();
    b.iter(|| a.bind(&c));
}
```

---

## 5. Open Questions

### Design Decisions Needed

1. **Const generics vs runtime parameters?**
   - Const: `GHRRAlgebra<512, 2>` - Zero-cost, compile-time checked
   - Runtime: `GHRRAlgebra::new(512, 2)` - Flexible, larger binary
   - **Recommendation:** Const generics with common type aliases

2. **Matrix storage format?**
   - Row-major flat array: `[Complex64; M*M]` - Simple, cache-friendly
   - 2D array: `[[Complex64; M]; M]` - Clearer indexing
   - External crate: `nalgebra::SMatrix<M, M>` - Rich linear algebra
   - **Recommendation:** Start with flat array, consider nalgebra later

3. **Polar decomposition algorithm?**
   - SVD-based: Most robust, O(m³)
   - Newton iteration: Fast for near-unitary, may not converge
   - **Recommendation:** SVD for correctness, optimize later if needed

4. **BLAS integration?**
   - For M ≤ 4: Hand-written is likely faster (no FFI overhead)
   - For M > 4: BLAS would help
   - **Recommendation:** Optional `ghrr-blas` feature

### Future Extensions

- **Sparse GHRR:** For very high D, only store/compute active components
- **GPU acceleration:** Matrix operations parallelize well
- **Automatic M selection:** Given desired capacity/accuracy tradeoff

---

## 6. Success Criteria

### Functional

- [ ] All `BindingAlgebra` trait methods implemented
- [ ] Non-commutativity demonstrated for M > 1
- [ ] Unbind correctly recovers bound values
- [ ] M=1 equivalent to existing FHRR implementation

### Performance

- [ ] Bind operation < 10μs for D=512, M=2
- [ ] Memory overhead < 2x vs equivalent FHRR
- [ ] Parallel operations scale with core count

### Quality

- [ ] 90%+ test coverage
- [ ] All public APIs documented
- [ ] Examples compile and run
- [ ] Benchmarks show expected scaling

---

## 7. Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1 | Core matrix ops | Matrix multiply, transpose, trace, polar decomp |
| 1-2 | Algebra impl | BindingAlgebra trait, basic tests |
| 2 | Random generation | Haar-distributed orthogonal matrices, Prop 4.1 |
| 2-3 | Serialization | serde support, feature flags |
| 3 | Docs & examples | Rustdoc, examples, benchmarks |

**Total: ~3 weeks** for complete implementation with tests and documentation.

---

## Appendix: Key Equations from Paper

### Proposition 4.1 (Quasi-orthogonality)

For base vectors Hⱼ = Qⱼ · Λⱼ where Q ∈ O(m) and Λ = diag(e^(iφ₁),...,e^(iφₘ)):

```
E[δ(H₁, H₂)] = 0  (zero expected similarity for independent random vectors)
Var[δ(H₁, H₂)] = 1/(mD)  (variance decreases with dimension)
```

### Corollary 4.1.1 (Similarity Preservation)

```
δ(H₁, H₂) ≈ δ(H₃ ⊗ H₁, H₃ ⊗ H₂)
```

Binding preserves relative similarities (crucial for memory retrieval).

### Capacity (Empirical, Figure 10)

For fixed total dimensionality D·m²:
- Bound vector capacity increases with M
- Single vector capacity roughly constant
- Sweet spot: M = 2-4 for most applications
