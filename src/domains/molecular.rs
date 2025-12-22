//! Molecular domain utilities.
//!
//! Encoders for molecular structures (SMILES, fingerprints).

use amari_fusion::TropicalDualClifford;

use crate::precision::MinuetFloat;

use super::DomainEncoder;

/// Encoder for molecular fingerprints.
///
/// Converts molecular fingerprint bit vectors into holographic representations.
pub struct FingerprintEncoder<T: MinuetFloat, const DIM: usize> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> FingerprintEncoder<T, DIM> {
    /// Create a new fingerprint encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for FingerprintEncoder<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> DomainEncoder<T, DIM> for FingerprintEncoder<T, DIM> {
    type Input = Vec<bool>;

    fn encode(&self, input: &Self::Input) -> TropicalDualClifford<T, DIM> {
        // Simple encoding: map each bit to a random direction
        // In full implementation, would use consistent random projections
        let mut result = TropicalDualClifford::bundling_zero();

        for (i, &bit) in input.iter().enumerate() {
            if bit {
                // Create a deterministic "random" direction for bit i
                let direction = TropicalDualClifford::from_seed(i as u64);
                result = result.bundle(&direction, T::one());
            }
        }

        // Normalize
        let mag = result.magnitude();
        if mag > T::MIN_POSITIVE {
            result = result.scale(T::one() / mag);
        }

        result
    }

    fn decode(&self, _repr: &TropicalDualClifford<T, DIM>) -> Option<Self::Input> {
        // Fingerprint decoding is lossy - not generally possible
        None
    }
}

/// SMILES string encoder.
///
/// Encodes SMILES molecular notation into holographic representations
/// using character n-grams and structural features.
pub struct SmilesEncoder<T: MinuetFloat, const DIM: usize> {
    /// N-gram size for character encoding.
    ngram_size: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> SmilesEncoder<T, DIM> {
    /// Create a new SMILES encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ngram_size: 3,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the n-gram size.
    #[must_use]
    pub fn with_ngram_size(mut self, size: usize) -> Self {
        self.ngram_size = size;
        self
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for SmilesEncoder<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> DomainEncoder<T, DIM> for SmilesEncoder<T, DIM> {
    type Input = String;

    fn encode(&self, input: &Self::Input) -> TropicalDualClifford<T, DIM> {
        let chars: Vec<char> = input.chars().collect();
        let mut result = TropicalDualClifford::bundling_zero();

        // Encode character n-grams
        for window in chars.windows(self.ngram_size) {
            let ngram: String = window.iter().collect();
            let hash = Self::hash_ngram(&ngram);
            let direction = TropicalDualClifford::from_seed(hash);
            result = result.bundle(&direction, T::one());
        }

        // Normalize
        let mag = result.magnitude();
        if mag > T::MIN_POSITIVE {
            result = result.scale(T::one() / mag);
        }

        result
    }

    fn decode(&self, _repr: &TropicalDualClifford<T, DIM>) -> Option<Self::Input> {
        // SMILES decoding is not directly possible
        None
    }
}

impl<T: MinuetFloat, const DIM: usize> SmilesEncoder<T, DIM> {
    fn hash_ngram(ngram: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ngram.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_encoding() {
        let encoder: FingerprintEncoder<f64, 64> = FingerprintEncoder::new();

        let fp1 = vec![true, false, true, true, false];
        let fp2 = vec![true, false, true, true, false];
        let fp3 = vec![false, true, false, false, true];

        let enc1 = encoder.encode(&fp1);
        let enc2 = encoder.encode(&fp2);
        let enc3 = encoder.encode(&fp3);

        // Same fingerprint should give same encoding
        assert!(enc1.similarity(&enc2) > 0.99);

        // Different fingerprints should be dissimilar
        assert!(enc1.similarity(&enc3) < 0.5);
    }

    #[test]
    fn smiles_encoding() {
        let encoder: SmilesEncoder<f64, 64> = SmilesEncoder::new();

        let aspirin = "CC(=O)OC1=CC=CC=C1C(=O)O".to_string();
        let similar = "CC(=O)OC1=CC=CC=C1C(=O)N".to_string(); // Aspirin with amide
        let different = "C1CCCCC1".to_string(); // Cyclohexane

        let enc_aspirin = encoder.encode(&aspirin);
        let enc_similar = encoder.encode(&similar);
        let enc_different = encoder.encode(&different);

        // Similar molecules should have higher similarity
        let sim_aspirin_similar = enc_aspirin.similarity(&enc_similar);
        let sim_aspirin_different = enc_aspirin.similarity(&enc_different);

        assert!(sim_aspirin_similar > sim_aspirin_different);
    }
}
