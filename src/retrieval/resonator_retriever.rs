//! Resonator-based retriever - uses cleanup network.

use amari_holographic::{BindingAlgebra, Resonator, ResonatorConfig};

use crate::error::{MinuetError, MinuetResult};
use crate::traits::{CleanupResult, RetrievalContext, Retriever};

/// A retriever that uses a resonator network for cleanup.
///
/// This retriever uses amari-holographic's Resonator to clean up
/// noisy retrieval results by iteratively projecting onto a codebook.
///
/// # Example
///
/// ```rust,ignore
/// use minuet::retrieval::ResonatorRetriever;
/// use minuet::encoding::HashMapCodebook;
/// use amari_holographic::ProductCliffordAlgebra;
///
/// type Algebra = ProductCliffordAlgebra<64>;
///
/// let codebook = HashMapCodebook::<Algebra>::new();
/// // ... populate codebook ...
///
/// let retriever = ResonatorRetriever::from_codebook(&codebook);
/// ```
pub struct ResonatorRetriever<A: BindingAlgebra> {
    resonator: Option<Resonator<A>>,
    config: ResonatorConfig,
}

impl<A: BindingAlgebra> Default for ResonatorRetriever<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra> ResonatorRetriever<A> {
    /// Create a new resonator retriever without a codebook.
    ///
    /// The codebook must be provided via the context at cleanup time.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resonator: None,
            config: ResonatorConfig::default(),
        }
    }

    /// Create with a pre-built resonator.
    #[must_use]
    pub fn with_resonator(resonator: Resonator<A>) -> Self {
        Self {
            resonator: Some(resonator),
            config: ResonatorConfig::default(),
        }
    }

    /// Create from a codebook (vec of symbols).
    ///
    /// # Errors
    ///
    /// Returns error if the codebook is empty.
    pub fn from_symbols(symbols: Vec<A>) -> MinuetResult<Self> {
        if symbols.is_empty() {
            return Err(MinuetError::config("Codebook cannot be empty"));
        }

        let config = ResonatorConfig::default();
        let resonator = Resonator::new(symbols, config.clone()).map_err(MinuetError::algebra)?;

        Ok(Self {
            resonator: Some(resonator),
            config,
        })
    }

    /// Set configuration.
    #[must_use]
    pub fn with_config(mut self, config: ResonatorConfig) -> Self {
        self.config = config;
        self
    }

    /// Set initial temperature (low = soft attention).
    #[must_use]
    pub fn initial_temperature(mut self, temp: f64) -> Self {
        self.config.initial_beta = temp;
        self
    }

    /// Set final temperature (high = hard selection).
    #[must_use]
    pub fn final_temperature(mut self, temp: f64) -> Self {
        self.config.final_beta = temp;
        self
    }

    /// Set max iterations.
    #[must_use]
    pub fn max_iterations(mut self, iters: usize) -> Self {
        self.config.max_iterations = iters;
        self
    }
}

impl<A: BindingAlgebra> Retriever for ResonatorRetriever<A> {
    type Algebra = A;

    fn cleanup(&self, raw: &A, context: &RetrievalContext<A>) -> MinuetResult<CleanupResult<A>> {
        // Try to use pre-built resonator, or build from context codebook
        let result = if let Some(ref resonator) = self.resonator {
            resonator.cleanup(raw)
        } else if let Some(ref codebook) = context.codebook {
            if codebook.is_empty() {
                return Ok(CleanupResult {
                    value: raw.clone(),
                    confidence: 0.5,
                    iterations: 0,
                    converged: false,
                    codebook_match: None,
                });
            }

            let temp_resonator = Resonator::new(codebook.clone(), self.config.clone())
                .map_err(MinuetError::algebra)?;
            temp_resonator.cleanup(raw)
        } else {
            // No codebook available, return raw
            return Ok(CleanupResult {
                value: raw.clone(),
                confidence: 0.5,
                iterations: 0,
                converged: false,
                codebook_match: None,
            });
        };

        Ok(CleanupResult {
            value: result.cleaned,
            confidence: result.final_similarity,
            iterations: result.iterations,
            converged: result.converged,
            codebook_match: Some(result.best_match_index),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>;

    #[test]
    fn cleanup_with_codebook() {
        // Create some symbols
        let symbols: Vec<TestAlgebra> = (0..5).map(|_| TestAlgebra::random_versor(2)).collect();

        let retriever = ResonatorRetriever::from_symbols(symbols.clone()).unwrap();

        // Cleanup should converge to one of the symbols
        let context = RetrievalContext::default();
        let result = retriever.cleanup(&symbols[2], &context).unwrap();

        assert!(result.converged);
        assert!(result.confidence > 0.9);
        assert!(result.codebook_match.is_some());
    }

    #[test]
    fn cleanup_without_codebook_returns_raw() {
        let retriever = ResonatorRetriever::<TestAlgebra>::new();
        let raw = TestAlgebra::random_versor(2);
        let context = RetrievalContext::default();

        let result = retriever.cleanup(&raw, &context).unwrap();

        // Without codebook, should return raw
        assert!(result.value.similarity(&raw) > 0.99);
    }
}
