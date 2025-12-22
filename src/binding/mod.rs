//! Binding algebra for holographic representations.
//!
//! This module provides operations for creating and manipulating holographic
//! bindings between representations, including:
//!
//! - **Algebra**: Core binding operations (bind, unbind, bundle)
//! - **Codebook**: Symbol vocabularies with stable representations
//! - **Transform**: Reified transformations that can be extracted and applied

mod algebra;
mod codebook;
mod transform;

pub use algebra::{BindingAlgebra, GradeProjection};
pub use codebook::{Codebook, StandardGenerator, SymbolGenerator, SymbolProperties};
pub use transform::{Transform, TransformMetadata};
