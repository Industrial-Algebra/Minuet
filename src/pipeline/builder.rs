//! Pipeline builder for fluent composition.

use amari_holographic::BindingAlgebra;

use crate::capacity::RejectPolicy;
use crate::encoding::HashMapCodebook;
use crate::error::{MinuetError, MinuetResult};
use crate::retrieval::DirectRetriever;
use crate::store::SimpleStore;
use crate::traits::{
    CapacityInfo, CapacityPolicy, Codebook, MemoryStore, RetrievalContext, RetrievalResult,
    Retriever, StoreReceipt,
};

/// Builder for composing holographic memory pipelines.
///
/// # Example
///
/// ```rust,ignore
/// use minuet::prelude::*;
/// use minuet::pipeline::PipelineBuilder;
/// use minuet::store::ShardedStore;
/// use minuet::retrieval::ResonatorRetriever;
///
/// let pipeline = PipelineBuilder::<ProductCliffordAlgebra<64>>::new()
///     .with_store(ShardedStore::with_shards(8))
///     .with_retriever(ResonatorRetriever::new())
///     .build()?;
/// ```
pub struct PipelineBuilder<A: BindingAlgebra> {
    store: Option<Box<dyn MemoryStore<Trace = crate::store::DenseTrace<A>, Algebra = A>>>,
    retriever: Option<Box<dyn Retriever<Algebra = A>>>,
    codebook: Option<Box<dyn Codebook<Algebra = A>>>,
    capacity_policy: Option<Box<dyn CapacityPolicy>>,
}

impl<A: BindingAlgebra + 'static> Default for PipelineBuilder<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: BindingAlgebra + 'static> PipelineBuilder<A> {
    /// Create a new pipeline builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: None,
            retriever: None,
            codebook: None,
            capacity_policy: None,
        }
    }

    /// Set the memory store.
    #[must_use]
    pub fn with_store<S>(mut self, store: S) -> Self
    where
        S: MemoryStore<Trace = crate::store::DenseTrace<A>, Algebra = A> + 'static,
    {
        self.store = Some(Box::new(store));
        self
    }

    /// Set the retriever.
    #[must_use]
    pub fn with_retriever<R>(mut self, retriever: R) -> Self
    where
        R: Retriever<Algebra = A> + 'static,
    {
        self.retriever = Some(Box::new(retriever));
        self
    }

    /// Set the codebook.
    #[must_use]
    pub fn with_codebook<C>(mut self, codebook: C) -> Self
    where
        C: Codebook<Algebra = A> + 'static,
    {
        self.codebook = Some(Box::new(codebook));
        self
    }

    /// Set the capacity policy.
    #[must_use]
    pub fn with_capacity_policy<P>(mut self, policy: P) -> Self
    where
        P: CapacityPolicy + 'static,
    {
        self.capacity_policy = Some(Box::new(policy));
        self
    }

    /// Build the pipeline.
    ///
    /// If no store is provided, uses SimpleStore.
    /// If no retriever is provided, uses DirectRetriever.
    /// If no codebook is provided, uses HashMapCodebook.
    /// If no capacity policy is provided, uses RejectPolicy.
    pub fn build(self) -> MinuetResult<Pipeline<A>> {
        Ok(Pipeline {
            store: self.store.unwrap_or_else(|| Box::new(SimpleStore::new())),
            retriever: self
                .retriever
                .unwrap_or_else(|| Box::new(DirectRetriever::new())),
            codebook: self
                .codebook
                .unwrap_or_else(|| Box::new(HashMapCodebook::new())),
            capacity_policy: self
                .capacity_policy
                .unwrap_or_else(|| Box::new(RejectPolicy::new())),
        })
    }
}

/// A composed holographic memory pipeline.
///
/// Combines a store, retriever, codebook, and capacity policy into
/// a unified interface.
pub struct Pipeline<A: BindingAlgebra> {
    store: Box<dyn MemoryStore<Trace = crate::store::DenseTrace<A>, Algebra = A>>,
    retriever: Box<dyn Retriever<Algebra = A>>,
    codebook: Box<dyn Codebook<Algebra = A>>,
    capacity_policy: Box<dyn CapacityPolicy>,
}

impl<A: BindingAlgebra> Pipeline<A> {
    /// Store a key-value association.
    ///
    /// # Errors
    ///
    /// Returns error if capacity policy rejects the store.
    pub fn store(&self, key: &A, value: &A) -> MinuetResult<StoreReceipt> {
        // Check capacity policy
        let info = self.store.capacity_info();
        if !self.capacity_policy.can_accept(&info) {
            return Err(MinuetError::CapacityExceeded);
        }

        self.store.store(key, value)
    }

    /// Retrieve with cleanup.
    pub fn retrieve(&self, key: &A) -> MinuetResult<RetrievalResult<A>> {
        let raw = self.store.retrieve(key)?;

        let context = RetrievalContext::default().with_codebook(self.codebook.all_symbols());

        let cleaned = self.retriever.cleanup(&raw.value, &context)?;

        Ok(RetrievalResult {
            value: cleaned.value,
            confidence: cleaned.confidence,
            attribution: raw.attribution,
        })
    }

    /// Get or create a symbol from the codebook.
    pub fn symbol(&self, name: &str) -> A {
        self.codebook.symbol(name)
    }

    /// Get capacity info.
    #[must_use]
    pub fn capacity_info(&self) -> CapacityInfo {
        self.store.capacity_info()
    }

    /// Clear the store.
    pub fn clear(&self) -> MinuetResult<()> {
        self.store.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amari_holographic::ProductCliffordAlgebra;

    type TestAlgebra = ProductCliffordAlgebra<8>;

    #[test]
    fn build_default_pipeline() {
        let pipeline = PipelineBuilder::<TestAlgebra>::new().build().unwrap();

        let key = pipeline.symbol("key");
        let value = pipeline.symbol("value");

        pipeline.store(&key, &value).unwrap();
        let result = pipeline.retrieve(&key).unwrap();

        // Should retrieve something
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn pipeline_respects_capacity_policy() {
        use crate::capacity::RejectPolicy;

        // Create a very strict policy
        let pipeline = PipelineBuilder::<TestAlgebra>::new()
            .with_capacity_policy(RejectPolicy::with_threshold(0.0))
            .build()
            .unwrap();

        let key = TestAlgebra::random_versor(2);
        let value = TestAlgebra::random_versor(2);

        // Should fail because threshold is 0
        let result = pipeline.store(&key, &value);
        assert!(result.is_err());
    }
}
