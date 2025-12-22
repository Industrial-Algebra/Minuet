//! Retrieval operations for holographic memory.
//!
//! This module provides:
//!
//! - **Resonator**: Cleanup networks for denoising retrieval results
//! - **Attribution**: Provenance tracking via dual gradients
//! - **Temperature**: Soft/hard retrieval interpolation

mod attribution;
mod resonator;
mod temperature;

pub use attribution::{Attribution, AttributionResult};
pub use resonator::{Resonator, ResonatorConfig, ResonatorResult};
pub use temperature::Temperature;
