//! Direct retriever - returns raw results without cleanup.

use amari_holographic::BindingAlgebra;

use crate::error::MinuetResult;
use crate::traits::{CleanupResult, RetrievalContext, Retriever};

/// A retriever that returns raw results without cleanup.
///
/// This is the simplest retriever - it just returns the raw unbind
/// result with a confidence estimate. Use this when:
/// - Memory utilization is low (high SNR)
/// - Cleanup is not needed
/// - You want maximum speed
///
/// # Example
///
/// ```rust,ignore
/// use minuet::retrieval::DirectRetriever;
/// use amari_holographic::ProductCliffordAlgebra;
///
/// type Algebra = ProductCliffordAlgebra<64>;
/// let retriever = DirectRetriever::<Algebra>::new();
/// ```
pub struct DirectRetriever<A: BindingAlgebra> {
    _marker: std::marker::PhantomData<A>,
}

impl<A: BindingAlgebra> Default for DirectRetriever<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> DirectRetriever<A> {
    /// Create a new direct retriever.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: BindingAlgebra> Retriever for DirectRetriever<A> {
    type Algebra = A;

    fn cleanup(&self, raw: &A, _context: &RetrievalContext<A>) -> MinuetResult<CleanupResult<A>> {
        // Just return the raw value with high confidence
        Ok(CleanupResult {
            value: raw.clone(),
            confidence: 1.0,
            iterations: 0,
            converged: true,
            codebook_match: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>;

    #[test]
    fn direct_returns_raw() {
        let retriever = DirectRetriever::<TestAlgebra>::new();
        let raw = TestAlgebra::random_versor(2);
        let context = RetrievalContext::default();

        let result = retriever.cleanup(&raw, &context).unwrap();

        assert!(result.value.similarity(&raw) > 0.99);
        assert!(result.converged);
        assert_eq!(result.iterations, 0);
    }
}
