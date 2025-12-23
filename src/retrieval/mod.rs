//! Retrieval strategies.
//!
//! This module provides strategies for cleaning up and interpreting
//! raw retrieval results from holographic memory.
//!
//! - [`DirectRetriever`](crate::retrieval::DirectRetriever): Returns raw results without cleanup
//! - [`ResonatorRetriever`](crate::retrieval::ResonatorRetriever): Uses resonator network for cleanup

mod direct;
mod resonator_retriever;

pub use direct::DirectRetriever;
pub use resonator_retriever::ResonatorRetriever;
