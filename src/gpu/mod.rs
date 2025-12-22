//! GPU acceleration for holographic memory operations.
//!
//! This module provides GPU-accelerated versions of computationally
//! intensive operations using WebGPU (wgpu).
//!
//! # Supported Operations
//!
//! The following operations benefit significantly from GPU acceleration:
//!
//! 1. **Batch binding**: Element-wise geometric products, highly parallel
//! 2. **Bundle reduction**: Parallel tree reduction for bundling
//! 3. **Similarity matrix**: N×M similarity computations
//! 4. **Resonator iteration**: Repeated similarity + weighted sum
//!
//! # Example
//!
//! ```rust,ignore
//! use minuet::gpu::{GpuContext, GpuMemoryStore};
//!
//! // Initialize GPU context
//! let ctx = GpuContext::new().await?;
//!
//! // Create GPU-accelerated memory store
//! let store = GpuMemoryStore::new(&ctx);
//!
//! // Operations automatically use GPU when beneficial
//! store.store_batch(&pairs)?;
//! ```

#[cfg(feature = "gpu")]
mod buffers;
#[cfg(feature = "gpu")]
mod dispatch;
#[cfg(feature = "gpu")]
mod kernels;

/// Trait for operations that can be GPU-accelerated.
pub trait GpuAccelerable {
    /// Estimated speedup from GPU execution.
    fn gpu_speedup_estimate(&self) -> f64;

    /// Minimum batch size where GPU is beneficial.
    fn gpu_batch_threshold(&self) -> usize;
}

/// Execution backend selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend {
    /// CPU execution (always available).
    #[default]
    Cpu,

    /// GPU execution (requires gpu feature).
    #[cfg(feature = "gpu")]
    Gpu,

    /// Automatically choose based on workload.
    Auto,
}

impl Backend {
    /// Check if this backend is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        match self {
            Backend::Cpu => true,
            #[cfg(feature = "gpu")]
            Backend::Gpu => {
                // Would check for GPU availability
                true
            }
            Backend::Auto => true,
        }
    }
}

/// GPU operation configuration.
#[derive(Clone, Debug)]
pub struct GpuConfig {
    /// Preferred backend.
    pub backend: Backend,

    /// Minimum batch size for GPU execution.
    pub min_batch_size: usize,

    /// Maximum GPU memory usage (bytes).
    pub max_memory: usize,

    /// Whether to use async GPU operations.
    pub async_dispatch: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Auto,
            min_batch_size: 1000,
            max_memory: 1024 * 1024 * 1024, // 1GB
            async_dispatch: true,
        }
    }
}

#[cfg(feature = "gpu")]
pub use buffers::GpuBufferPool;
#[cfg(feature = "gpu")]
pub use dispatch::GpuDispatcher;
#[cfg(feature = "gpu")]
pub use kernels::ComputeKernels;

/// GPU context placeholder (full implementation behind feature gate).
#[cfg(feature = "gpu")]
pub struct GpuContext {
    // wgpu device, queue, etc.
}

#[cfg(feature = "gpu")]
impl GpuContext {
    /// Create a new GPU context.
    pub async fn new() -> crate::error::Result<Self> {
        todo!("GPU context initialization")
    }

    /// Check if GPU is available.
    pub fn is_available() -> bool {
        // Would check for GPU
        false
    }
}
