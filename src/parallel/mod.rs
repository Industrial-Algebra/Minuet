//! Parallel operations for holographic memory.
//!
//! This module provides:
//!
//! - **Batch**: Parallel batch binding and bundling operations
//! - **Sharded**: Sharded memory for increased capacity
//! - **Merge**: Parallel trace merging

pub mod batch;
mod merge;
mod sharded;

pub use batch::{bind_batch_parallel, bundle_parallel, similarities_parallel};
pub use merge::{merge_traces_parallel, MergeStrategy};
pub use sharded::{ShardHasher, ShardedMemory};
