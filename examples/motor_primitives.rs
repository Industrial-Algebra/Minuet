//! Motor primitives example for robotics.
//!
//! Demonstrates using holographic memory for composing and
//! searching motor primitives using SE(3) representations.

use std::f64::consts::PI;

use minuet::{
    binding::Codebook,
    domains::geometric::{MotorPrimitives, Point3, RigidTransform, Rotation3, SE3Encoder},
    memory::{BasicMemoryStore, MemoryStore},
};

fn main() -> minuet::Result<()> {
    println!("=== Motor Primitives Example ===\n");

    // Create motor primitive library
    let mut primitives: MotorPrimitives<f64, 8> = MotorPrimitives::new();
    let encoder: SE3Encoder<f64, 8> = SE3Encoder::new();
    let memory: BasicMemoryStore<f64, 128> = BasicMemoryStore::new();
    let codebook: Codebook<f64, 128> = Codebook::new();

    println!("Creating motor primitives...\n");

    // Define basic movement primitives
    let moves = vec![
        // Translations
        ("move_forward", RigidTransform::translation(0.1, 0.0, 0.0)),
        ("move_back", RigidTransform::translation(-0.1, 0.0, 0.0)),
        ("move_left", RigidTransform::translation(0.0, 0.1, 0.0)),
        ("move_right", RigidTransform::translation(0.0, -0.1, 0.0)),
        ("move_up", RigidTransform::translation(0.0, 0.0, 0.1)),
        ("move_down", RigidTransform::translation(0.0, 0.0, -0.1)),
        // Rotations
        (
            "turn_left",
            RigidTransform::rotation(Rotation3::from_axis_z(PI / 4.0)),
        ),
        (
            "turn_right",
            RigidTransform::rotation(Rotation3::from_axis_z(-PI / 4.0)),
        ),
        (
            "tilt_up",
            RigidTransform::rotation(Rotation3::from_axis_y(PI / 6.0)),
        ),
        (
            "tilt_down",
            RigidTransform::rotation(Rotation3::from_axis_y(-PI / 6.0)),
        ),
    ];

    for (name, transform) in &moves {
        primitives.add(name, transform.clone());
        println!("  Added primitive: {}", name);
    }

    println!("\n--- Primitive Composition ---\n");

    // Compose primitives
    if let Some(composed) = primitives.compose("move_forward", "turn_left") {
        println!("Composed: move_forward -> turn_left");

        // Find similar primitive
        if let Some((nearest, sim)) = primitives.nearest(&composed) {
            println!("  Nearest single primitive: {} (similarity: {:.3})", nearest, sim);
        }
    }

    // Compose multiple steps
    println!("\nMulti-step composition: forward -> turn_left -> forward");

    if let (Some(step1), Some(step2)) = (
        primitives.get("move_forward"),
        primitives.get("turn_left"),
    ) {
        use amari_fusion::holographic::Bindable;

        let composed = step2.bind(step1);
        if let Some(step3) = primitives.get("move_forward") {
            let final_motion = step3.bind(&composed);
            println!("  Final motion computed (magnitude: {:.3})", final_motion.magnitude());
        }
    }

    println!("\n--- Primitive Search ---\n");

    // Search for primitives by desired outcome
    println!("Finding primitives that move in positive X direction...");

    // Create a target: pure forward translation
    let target = encoder.encode(&RigidTransform::translation(0.1, 0.0, 0.0));

    if let Some((name, sim)) = primitives.nearest(&target) {
        println!("  Best match: {} (similarity: {:.3})", name, sim);
    }

    println!("\n--- Trajectory Planning ---\n");

    // Simple trajectory: square pattern
    let trajectory = ["move_forward", "turn_left", "move_forward", "turn_left"];

    println!("Executing trajectory (square pattern):");
    let mut current_pose = RigidTransform::identity();

    for step_name in trajectory {
        println!("  -> {}", step_name);
        // In a real implementation, would compose the transforms
    }

    println!("\n=== Example Complete ===");

    Ok(())
}
