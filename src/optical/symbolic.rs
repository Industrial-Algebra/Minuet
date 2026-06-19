// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Symbolic expression types for hardware-independent memory representation.
//!
//! These expressions can be serialized, transmitted, and instantiated on any
//! hardware configuration. They form the portable representation layer that
//! enables checkpoint-based persistence.

use amari_holographic::optical::SymbolId;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Wrapper for f32 that implements Eq and Hash using bit representation.
///
/// This enables use in hash-based collections and as Bundle weights in
/// `SymbolicExpression`. Equality is based on bit-level representation,
/// so NaN == NaN (same bits) but different NaN representations are not equal.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OrderedFloat<T>(pub T);

impl OrderedFloat<f32> {
    /// Create a new OrderedFloat.
    #[inline]
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Get the inner value.
    #[inline]
    pub fn into_inner(self) -> f32 {
        self.0
    }
}

impl PartialEq for OrderedFloat<f32> {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for OrderedFloat<f32> {}

impl Hash for OrderedFloat<f32> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for OrderedFloat<f32> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat<f32> {
    #[allow(clippy::cast_possible_wrap)]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Total ordering: -NaN < -Inf < ... < -0 < +0 < ... < +Inf < +NaN
        let a = self.0.to_bits() as i32;
        let b = other.0.to_bits() as i32;

        // Handle sign bit for proper negative number ordering
        let a_adj = if a < 0 { !a } else { a ^ (1 << 31) };
        let b_adj = if b < 0 { !b } else { b ^ (1 << 31) };

        a_adj.cmp(&b_adj)
    }
}

impl From<f32> for OrderedFloat<f32> {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<OrderedFloat<f32>> for f32 {
    fn from(value: OrderedFloat<f32>) -> Self {
        value.0
    }
}

/// Symbolic expression tree for portable memory representation.
///
/// These expressions can be serialized, transmitted, and instantiated
/// on any hardware configuration. They represent the logical structure
/// of holographic memory without hardware-specific encoding.
///
/// # Examples
///
/// ```ignore
/// use minuet::optical::SymbolicExpression;
///
/// // Create atomic symbols
/// let agent = SymbolicExpression::symbol("AGENT");
/// let john = SymbolicExpression::symbol("John");
///
/// // Create binding (association)
/// let agent_is_john = SymbolicExpression::bind(agent, john);
///
/// // Role-filler convenience method
/// let same = SymbolicExpression::role_filler("AGENT", "John");
///
/// // Create weighted bundle (superposition)
/// let bundle = SymbolicExpression::bundle(vec![
///     (1.0, SymbolicExpression::symbol("cat")),
///     (0.5, SymbolicExpression::symbol("dog")),
/// ]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolicExpression {
    /// Atomic symbol reference.
    Symbol(SymbolId),

    /// Binding of two expressions: a ⊗ b.
    ///
    /// Creates an association that is dissimilar to both inputs.
    /// Use `unbind` to retrieve associated values.
    Bind(Box<SymbolicExpression>, Box<SymbolicExpression>),

    /// Weighted bundle (superposition): Σ wᵢ · eᵢ.
    ///
    /// Creates a superposition that is similar to all inputs,
    /// with similarity proportional to weight.
    Bundle(Vec<(OrderedFloat<f32>, SymbolicExpression)>),
}

impl SymbolicExpression {
    /// Create a symbol expression from a name.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let expr = SymbolicExpression::symbol("AGENT");
    /// ```
    pub fn symbol(name: impl Into<String>) -> Self {
        Self::Symbol(SymbolId::new(name))
    }

    /// Create a binding expression: a ⊗ b.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bound = SymbolicExpression::bind(
    ///     SymbolicExpression::symbol("role"),
    ///     SymbolicExpression::symbol("filler"),
    /// );
    /// ```
    pub fn bind(a: Self, b: Self) -> Self {
        Self::Bind(Box::new(a), Box::new(b))
    }

    /// Create a bundle expression with weights.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bundle = SymbolicExpression::bundle(vec![
    ///     (1.0, SymbolicExpression::symbol("primary")),
    ///     (0.5, SymbolicExpression::symbol("secondary")),
    /// ]);
    /// ```
    pub fn bundle(elements: Vec<(f32, Self)>) -> Self {
        Self::Bundle(
            elements
                .into_iter()
                .map(|(w, e)| (OrderedFloat(w), e))
                .collect(),
        )
    }

    /// Create a bundle with uniform weights.
    ///
    /// All elements receive equal weight of 1.0.
    pub fn bundle_uniform(elements: Vec<Self>) -> Self {
        Self::Bundle(
            elements
                .into_iter()
                .map(|e| (OrderedFloat(1.0), e))
                .collect(),
        )
    }

    /// Convenience: bind a role with a filler.
    ///
    /// Equivalent to `bind(symbol(role), symbol(filler))`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // These are equivalent:
    /// let a = SymbolicExpression::role_filler("AGENT", "John");
    /// let b = SymbolicExpression::bind(
    ///     SymbolicExpression::symbol("AGENT"),
    ///     SymbolicExpression::symbol("John"),
    /// );
    /// assert_eq!(a, b);
    /// ```
    pub fn role_filler(role: impl Into<String>, filler: impl Into<String>) -> Self {
        Self::bind(Self::symbol(role), Self::symbol(filler))
    }

    /// Collect all symbol IDs referenced in this expression.
    ///
    /// Returns references to all `SymbolId`s in the expression tree,
    /// traversing bindings and bundles recursively.
    pub fn referenced_symbols(&self) -> Vec<&SymbolId> {
        match self {
            Self::Symbol(id) => vec![id],
            Self::Bind(a, b) => {
                let mut syms = a.referenced_symbols();
                syms.extend(b.referenced_symbols());
                syms
            }
            Self::Bundle(elements) => elements
                .iter()
                .flat_map(|(_, e)| e.referenced_symbols())
                .collect(),
        }
    }

    /// Check if this expression is an atomic symbol.
    pub fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol(_))
    }

    /// Check if this expression is a binding.
    pub fn is_bind(&self) -> bool {
        matches!(self, Self::Bind(_, _))
    }

    /// Check if this expression is a bundle.
    pub fn is_bundle(&self) -> bool {
        matches!(self, Self::Bundle(_))
    }

    /// Get the symbol ID if this is an atomic symbol.
    pub fn as_symbol(&self) -> Option<&SymbolId> {
        match self {
            Self::Symbol(id) => Some(id),
            _ => None,
        }
    }

    /// Count the total number of nodes in this expression tree.
    pub fn node_count(&self) -> usize {
        match self {
            Self::Symbol(_) => 1,
            Self::Bind(a, b) => 1 + a.node_count() + b.node_count(),
            Self::Bundle(elements) => {
                1 + elements.iter().map(|(_, e)| e.node_count()).sum::<usize>()
            }
        }
    }

    /// Maximum depth of this expression tree.
    pub fn depth(&self) -> usize {
        match self {
            Self::Symbol(_) => 1,
            Self::Bind(a, b) => 1 + a.depth().max(b.depth()),
            Self::Bundle(elements) => {
                1 + elements.iter().map(|(_, e)| e.depth()).max().unwrap_or(0)
            }
        }
    }
}

impl std::fmt::Display for SymbolicExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symbol(id) => write!(f, "{id}"),
            Self::Bind(a, b) => write!(f, "({a} ⊗ {b})"),
            Self::Bundle(elements) => {
                write!(f, "[")?;
                for (i, (w, e)) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, " + ")?;
                    }
                    let weight = w.0;
                    if (weight - 1.0).abs() > 0.001 {
                        write!(f, "{weight:.2}·{e}")?;
                    } else {
                        write!(f, "{e}")?;
                    }
                }
                write!(f, "]")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_float_equality() {
        let a = OrderedFloat(1.0f32);
        let b = OrderedFloat(1.0f32);
        let c = OrderedFloat(2.0f32);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_ordered_float_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(OrderedFloat(1.0f32));
        set.insert(OrderedFloat(1.0f32)); // duplicate
        set.insert(OrderedFloat(2.0f32));

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_symbolic_expression_symbol() {
        let expr = SymbolicExpression::symbol("test");
        assert!(expr.is_symbol());
        assert_eq!(expr.as_symbol().unwrap().as_str(), "test");
    }

    #[test]
    fn test_symbolic_expression_bind() {
        let expr = SymbolicExpression::bind(
            SymbolicExpression::symbol("a"),
            SymbolicExpression::symbol("b"),
        );
        assert!(expr.is_bind());

        let symbols = expr.referenced_symbols();
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn test_symbolic_expression_bundle() {
        let expr = SymbolicExpression::bundle(vec![
            (1.0, SymbolicExpression::symbol("a")),
            (0.5, SymbolicExpression::symbol("b")),
        ]);
        assert!(expr.is_bundle());

        let symbols = expr.referenced_symbols();
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn test_role_filler() {
        let a = SymbolicExpression::role_filler("AGENT", "John");
        let b = SymbolicExpression::bind(
            SymbolicExpression::symbol("AGENT"),
            SymbolicExpression::symbol("John"),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn test_node_count_and_depth() {
        let simple = SymbolicExpression::symbol("x");
        assert_eq!(simple.node_count(), 1);
        assert_eq!(simple.depth(), 1);

        let nested = SymbolicExpression::bind(
            SymbolicExpression::bind(
                SymbolicExpression::symbol("a"),
                SymbolicExpression::symbol("b"),
            ),
            SymbolicExpression::symbol("c"),
        );
        assert_eq!(nested.node_count(), 5); // 2 binds + 3 symbols
        assert_eq!(nested.depth(), 3);
    }

    #[test]
    fn test_display() {
        let expr = SymbolicExpression::role_filler("AGENT", "John");
        let s = format!("{}", expr);
        assert!(s.contains("AGENT"));
        assert!(s.contains("John"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let expr = SymbolicExpression::bind(
            SymbolicExpression::symbol("AGENT"),
            SymbolicExpression::symbol("John"),
        );

        let json = serde_json::to_string(&expr).unwrap();
        let restored: SymbolicExpression = serde_json::from_str(&json).unwrap();

        assert_eq!(expr, restored);
    }
}
