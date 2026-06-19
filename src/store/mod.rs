// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Memory store implementations.
//!
//! This module provides various store implementations for holographic memory:
//!
//! - [`DenseTrace`](crate::store::DenseTrace): The fundamental trace representation
//! - [`SimpleStore`](crate::store::SimpleStore): Single-trace store for simple use cases
//! - [`ShardedStore`](crate::store::ShardedStore): Hash-sharded store for larger capacity

mod sharded;
mod simple;
mod trace;

pub use sharded::ShardedStore;
pub use simple::SimpleStore;
pub use trace::DenseTrace;
