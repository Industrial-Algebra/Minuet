//! Geometric domain utilities.
//!
//! SE(3) motor primitives and spatial relationship encoding.

use std::marker::PhantomData;

use amari_fusion::holographic::{Bindable, TropicalDualClifford};

use crate::precision::MinuetFloat;

use super::DomainEncoder;

/// A 3D point.
#[derive(Debug, Clone, Copy)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    /// Create a new point.
    #[must_use]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Origin.
    #[must_use]
    pub fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Distance to another point.
    #[must_use]
    pub fn distance(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A 3D rotation (quaternion representation).
#[derive(Debug, Clone, Copy)]
pub struct Rotation3 {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Rotation3 {
    /// Create from quaternion components.
    #[must_use]
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    /// Identity rotation.
    #[must_use]
    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    /// Rotation around X axis.
    #[must_use]
    pub fn from_axis_x(angle: f64) -> Self {
        let half = angle / 2.0;
        Self::new(half.cos(), half.sin(), 0.0, 0.0)
    }

    /// Rotation around Y axis.
    #[must_use]
    pub fn from_axis_y(angle: f64) -> Self {
        let half = angle / 2.0;
        Self::new(half.cos(), 0.0, half.sin(), 0.0)
    }

    /// Rotation around Z axis.
    #[must_use]
    pub fn from_axis_z(angle: f64) -> Self {
        let half = angle / 2.0;
        Self::new(half.cos(), 0.0, 0.0, half.sin())
    }

    /// Compose two rotations.
    #[must_use]
    pub fn compose(&self, other: &Self) -> Self {
        Self::new(
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        )
    }
}

/// SE(3) rigid body transformation.
#[derive(Debug, Clone, Copy)]
pub struct RigidTransform {
    pub rotation: Rotation3,
    pub translation: Point3,
}

impl RigidTransform {
    /// Create a new rigid transform.
    #[must_use]
    pub fn new(rotation: Rotation3, translation: Point3) -> Self {
        Self { rotation, translation }
    }

    /// Identity transform.
    #[must_use]
    pub fn identity() -> Self {
        Self::new(Rotation3::identity(), Point3::origin())
    }

    /// Pure translation.
    #[must_use]
    pub fn translation(x: f64, y: f64, z: f64) -> Self {
        Self::new(Rotation3::identity(), Point3::new(x, y, z))
    }

    /// Pure rotation.
    #[must_use]
    pub fn rotation(rot: Rotation3) -> Self {
        Self::new(rot, Point3::origin())
    }
}

/// Encoder for SE(3) rigid body transformations.
///
/// Encodes SE(3) elements (rotations + translations) as motors in
/// Clifford algebra, which are naturally represented in TDC.
pub struct SE3Encoder<T, const DIM: usize> {
    _phantom: PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> SE3Encoder<T, DIM> {
    /// Create a new SE(3) encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Encode a rotation as a rotor.
    fn encode_rotation(&self, rot: &Rotation3) -> TropicalDualClifford<T, DIM> {
        // Quaternion maps to rotor in Cl(3,0)
        // In higher dimensions, we embed in the first 4 components
        let coeffs = [rot.w, rot.x, rot.y, rot.z];
        TropicalDualClifford::from_quaternion(&coeffs)
    }

    /// Encode a translation as a translator.
    fn encode_translation(&self, trans: &Point3) -> TropicalDualClifford<T, DIM> {
        // Translation maps to translator: 1 + (1/2)(t_x e_{01} + t_y e_{02} + t_z e_{03})
        // where e_{0i} are dual basis elements
        TropicalDualClifford::from_translation(trans.x, trans.y, trans.z)
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for SE3Encoder<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> DomainEncoder<T, DIM> for SE3Encoder<T, DIM> {
    type Input = RigidTransform;

    fn encode(&self, input: &Self::Input) -> TropicalDualClifford<T, DIM> {
        // Motor = Translator * Rotor
        let rotor = self.encode_rotation(&input.rotation);
        let translator = self.encode_translation(&input.translation);

        // Compose as motor
        translator.bind(&rotor)
    }

    fn decode(&self, repr: &TropicalDualClifford<T, DIM>) -> Option<Self::Input> {
        // Extract rotation and translation from motor
        // This is more complex in practice
        Some(RigidTransform::identity()) // Placeholder
    }
}

/// Motor primitive library for robotics.
///
/// Pre-computed motor primitives for common movements.
pub struct MotorPrimitives<T, const DIM: usize> {
    encoder: SE3Encoder<T, DIM>,
    primitives: Vec<(String, TropicalDualClifford<T, DIM>)>,
}

impl<T: MinuetFloat, const DIM: usize> MotorPrimitives<T, DIM> {
    /// Create a new motor primitive library.
    #[must_use]
    pub fn new() -> Self {
        Self {
            encoder: SE3Encoder::new(),
            primitives: Vec::new(),
        }
    }

    /// Add a named primitive.
    pub fn add(&mut self, name: &str, transform: RigidTransform) {
        let encoded = self.encoder.encode(&transform);
        self.primitives.push((name.to_string(), encoded));
    }

    /// Get a primitive by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&TropicalDualClifford<T, DIM>> {
        self.primitives
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| p)
    }

    /// Compose two primitives by name.
    #[must_use]
    pub fn compose(&self, first: &str, second: &str) -> Option<TropicalDualClifford<T, DIM>> {
        let p1 = self.get(first)?;
        let p2 = self.get(second)?;
        Some(p2.bind(p1))
    }

    /// Find the most similar primitive.
    #[must_use]
    pub fn nearest(&self, query: &TropicalDualClifford<T, DIM>) -> Option<(&str, f64)> {
        self.primitives
            .iter()
            .map(|(name, prim)| (name.as_str(), query.similarity(prim).to_f64().unwrap_or(0.0)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for MotorPrimitives<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_transform() {
        let encoder: SE3Encoder<f64, 8> = SE3Encoder::new();
        let identity = RigidTransform::identity();

        let encoded = encoder.encode(&identity);

        // Identity should be close to binding identity
        let binding_id = TropicalDualClifford::binding_identity();
        assert!(encoded.similarity(&binding_id) > 0.9);
    }

    #[test]
    fn rotation_composition() {
        let encoder: SE3Encoder<f64, 8> = SE3Encoder::new();

        let rot_x = RigidTransform::rotation(Rotation3::from_axis_x(0.5));
        let rot_y = RigidTransform::rotation(Rotation3::from_axis_y(0.5));

        let enc_x = encoder.encode(&rot_x);
        let enc_y = encoder.encode(&rot_y);

        // Composition should be different from either
        let composed = enc_y.bind(&enc_x);
        assert!(composed.similarity(&enc_x) < 0.99);
        assert!(composed.similarity(&enc_y) < 0.99);
    }
}
