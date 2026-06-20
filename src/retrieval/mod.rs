// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only
//! Retrieval strategies.
//!
//! This module provides strategies for cleaning up and interpreting
//! raw retrieval results from holographic memory.
//!
//! - [`DirectRetriever`](crate::retrieval::DirectRetriever): Returns raw results without cleanup
//! - [`ResonatorRetriever`](crate::retrieval::ResonatorRetriever): Uses resonator network for cleanup
//! - [`Temperature`](crate::retrieval::Temperature) / [`TemperatureSchedule`](crate::retrieval::TemperatureSchedule): Annealed cleanup schedules (restored WS 2)

mod direct;
mod resonator_retriever;
mod temperature;

pub use direct::DirectRetriever;
pub use resonator_retriever::ResonatorRetriever;
pub use temperature::{Temperature, TemperatureSchedule};
