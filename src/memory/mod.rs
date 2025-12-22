//! Core holographic memory structures and operations.
//!
//! This module provides the fundamental abstractions for storing and retrieving
//! information in holographic form using tropical-dual-Clifford algebra.

mod capacity;
mod query;
mod store;
mod trace;

pub use capacity::{CapacityInfo, CapacityTracker};
pub use query::{BitMask, CleanupStrategy, Query, QueryPattern, QueryResult, QueryStats, RankedResult};
pub use store::{MergeResult, MemoryStore, StoreReceipt};
pub use trace::MemoryTrace;
