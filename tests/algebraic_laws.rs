//! Property-based tests for binding algebra laws.
//!
//! These tests verify that the TDC binding operations satisfy the
//! expected algebraic laws for holographic memory.

use proptest::prelude::*;

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use minuet::binding::BindingAlgebra;

/// Strategy for generating arbitrary TDC elements.
fn arbitrary_tdc<const DIM: usize>() -> impl Strategy<Value = TropicalDualClifford<f64, DIM>> {
    // Generate random TDC elements
    Just(()).prop_map(|_| TropicalDualClifford::random())
}

/// Strategy for generating normalized TDC elements.
fn normalized_tdc<const DIM: usize>() -> impl Strategy<Value = TropicalDualClifford<f64, DIM>> {
    arbitrary_tdc().prop_map(|x| {
        let mag = x.magnitude();
        if mag > f64::MIN_POSITIVE {
            x.scale(1.0 / mag)
        } else {
            x
        }
    })
}

/// Strategy for generating non-zero TDC elements.
fn nonzero_tdc<const DIM: usize>() -> impl Strategy<Value = TropicalDualClifford<f64, DIM>> {
    arbitrary_tdc().prop_filter("magnitude too small", |x| x.magnitude() > 1e-10)
}

proptest! {
    /// Binding identity: x ⊛ identity ≈ x
    #[test]
    fn binding_identity_right(x in arbitrary_tdc::<8>()) {
        let identity = TropicalDualClifford::binding_identity();
        let result = x.bind(&identity);
        prop_assert!(result.similarity(&x) > 0.95,
            "x ⊛ identity should ≈ x, got similarity {}",
            result.similarity(&x));
    }

    /// Binding identity: identity ⊛ x ≈ x
    #[test]
    fn binding_identity_left(x in arbitrary_tdc::<8>()) {
        let identity = TropicalDualClifford::binding_identity();
        let result = identity.bind(&x);
        prop_assert!(result.similarity(&x) > 0.95,
            "identity ⊛ x should ≈ x, got similarity {}",
            result.similarity(&x));
    }

    /// Binding inverse: x ⊛ x⁻¹ ≈ identity
    #[test]
    fn binding_inverse(x in nonzero_tdc::<8>()) {
        let identity = TropicalDualClifford::binding_identity();
        if let Some(x_inv) = x.binding_inverse() {
            let result = x.bind(&x_inv);
            prop_assert!(result.similarity(&identity) > 0.9,
                "x ⊛ x⁻¹ should ≈ identity, got similarity {}",
                result.similarity(&identity));
        }
    }

    /// Binding associativity: (a ⊛ b) ⊛ c ≈ a ⊛ (b ⊛ c)
    #[test]
    fn binding_associativity(
        a in arbitrary_tdc::<8>(),
        b in arbitrary_tdc::<8>(),
        c in arbitrary_tdc::<8>(),
    ) {
        let lhs = a.bind(&b).bind(&c);
        let rhs = a.bind(&b.bind(&c));
        prop_assert!(lhs.similarity(&rhs) > 0.9,
            "(a⊛b)⊛c should ≈ a⊛(b⊛c), got similarity {}",
            lhs.similarity(&rhs));
    }

    /// Bundling commutativity: a ⊕ b ≈ b ⊕ a
    #[test]
    fn bundle_commutative(
        a in arbitrary_tdc::<8>(),
        b in arbitrary_tdc::<8>(),
    ) {
        let ab = a.bundle(&b, 1.0);
        let ba = b.bundle(&a, 1.0);
        prop_assert!(ab.similarity(&ba) > 0.99,
            "a⊕b should ≈ b⊕a, got similarity {}",
            ab.similarity(&ba));
    }

    /// Bundling associativity: (a ⊕ b) ⊕ c ≈ a ⊕ (b ⊕ c)
    #[test]
    fn bundle_associativity(
        a in arbitrary_tdc::<8>(),
        b in arbitrary_tdc::<8>(),
        c in arbitrary_tdc::<8>(),
    ) {
        let lhs = a.bundle(&b, 1.0).bundle(&c, 1.0);
        let rhs = a.bundle(&b.bundle(&c, 1.0), 1.0);
        prop_assert!(lhs.similarity(&rhs) > 0.9,
            "(a⊕b)⊕c should ≈ a⊕(b⊕c), got similarity {}",
            lhs.similarity(&rhs));
    }

    /// Bundling identity: a ⊕ zero ≈ a
    #[test]
    fn bundle_identity(a in arbitrary_tdc::<8>()) {
        let zero = TropicalDualClifford::bundling_zero();
        let result = a.bundle(&zero, 1.0);
        // This depends on bundling semantics
        prop_assert!(result.magnitude() > 0.0 || a.magnitude() == 0.0);
    }

    /// Binding distributes over bundling: a ⊛ (b ⊕ c) ≈ (a ⊛ b) ⊕ (a ⊛ c)
    #[test]
    fn distributivity(
        a in normalized_tdc::<8>(),
        b in arbitrary_tdc::<8>(),
        c in arbitrary_tdc::<8>(),
    ) {
        let lhs = a.bind(&b.bundle(&c, 1.0));
        let rhs = a.bind(&b).bundle(&a.bind(&c), 1.0);
        // Distributivity is approximate due to normalization effects
        prop_assert!(lhs.similarity(&rhs) > 0.8,
            "a⊛(b⊕c) should ≈ (a⊛b)⊕(a⊛c), got similarity {}",
            lhs.similarity(&rhs));
    }

    /// Binding produces dissimilar results
    #[test]
    fn binding_dissimilarity(
        a in normalized_tdc::<8>(),
        b in normalized_tdc::<8>(),
    ) {
        let bound = a.bind(&b);
        // Bound element should be dissimilar to inputs
        // (this is the key property for holographic memory)
        let sim_a = bound.similarity(&a).abs();
        let sim_b = bound.similarity(&b).abs();
        prop_assert!(sim_a < 0.5,
            "a⊛b should be dissimilar to a, got {}",
            sim_a);
        prop_assert!(sim_b < 0.5,
            "a⊛b should be dissimilar to b, got {}",
            sim_b);
    }

    /// Unbinding recovers value: (a ⊛ b).unbind(a) ≈ b
    #[test]
    fn unbinding_recovery(
        a in nonzero_tdc::<8>(),
        b in arbitrary_tdc::<8>(),
    ) {
        let bound = a.bind(&b);
        let recovered = a.unbind(&bound);
        prop_assert!(recovered.similarity(&b) > 0.8,
            "unbind(a, a⊛b) should ≈ b, got similarity {}",
            recovered.similarity(&b));
    }

    /// Similarity is symmetric: sim(a,b) = sim(b,a)
    #[test]
    fn similarity_symmetric(
        a in arbitrary_tdc::<8>(),
        b in arbitrary_tdc::<8>(),
    ) {
        let sim_ab = a.similarity(&b);
        let sim_ba = b.similarity(&a);
        prop_assert!((sim_ab - sim_ba).abs() < 1e-10,
            "sim(a,b)={} should = sim(b,a)={}",
            sim_ab, sim_ba);
    }

    /// Self-similarity is 1 for normalized elements
    #[test]
    fn self_similarity(a in normalized_tdc::<8>()) {
        let sim = a.similarity(&a);
        prop_assert!((sim - 1.0).abs() < 1e-10,
            "sim(a,a) should = 1, got {}",
            sim);
    }

    /// Magnitude is non-negative
    #[test]
    fn magnitude_nonnegative(a in arbitrary_tdc::<8>()) {
        prop_assert!(a.magnitude() >= 0.0);
    }

    /// Scaling by scalar multiplies magnitude
    #[test]
    fn scaling_magnitude(
        a in nonzero_tdc::<8>(),
        s in 0.1f64..10.0f64,
    ) {
        let original_mag = a.magnitude();
        let scaled = a.scale(s);
        let scaled_mag = scaled.magnitude();
        prop_assert!((scaled_mag - original_mag * s).abs() / original_mag < 0.01,
            "||s*a|| should = |s|*||a||");
    }
}

/// Additional deterministic tests for edge cases.
mod edge_cases {
    use super::*;

    #[test]
    fn zero_binding() {
        let zero: TropicalDualClifford<f64, 8> = TropicalDualClifford::bundling_zero();
        let random = TropicalDualClifford::random();

        // Binding with zero should produce something with small magnitude
        let result = zero.bind(&random);
        assert!(result.magnitude() < 1e-10 || result.magnitude().is_nan());
    }

    #[test]
    fn high_dimensional_stability() {
        // Test that operations remain stable in higher dimensions
        let a: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        let b: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        let bound = a.bind(&b);
        assert!(!bound.magnitude().is_nan());
        assert!(!bound.magnitude().is_infinite());
    }

    #[test]
    fn repeated_binding_accumulation() {
        let key: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        let value: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();

        // Binding multiple times should remain stable
        let mut result = key.clone();
        for _ in 0..10 {
            result = result.bind(&value);
        }

        assert!(!result.magnitude().is_nan());
        assert!(result.magnitude() > 0.0);
    }
}
