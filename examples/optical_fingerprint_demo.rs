//! T-Matrix Fingerprinting Demo
//!
//! Demonstrates hardware validation using T-matrix fingerprinting.
//! This technique enables fast detection of hardware drift or replacement
//! without requiring full recalibration.
//!
//! Run with: cargo run --example optical_fingerprint_demo --features optical

use minuet::optical::{
    FingerprintValidation, MockOpticalHardware, OpticalHardware, TMatrixFingerprint,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== T-Matrix Fingerprinting Demo ===\n");

    // Create initial hardware
    let mut hardware = MockOpticalHardware::new(42);
    println!("Created hardware: {}", hardware.id());
    println!("  Temperature: {:.1}°C", hardware.temperature()?);
    println!("  Modes: {}\n", hardware.n_modes());

    // Capture fingerprint (5 probe patterns)
    println!("Capturing T-matrix fingerprint...");
    let fingerprint = TMatrixFingerprint::capture(&mut hardware, 5)?;
    println!("  Captured {} probe responses", fingerprint.responses.len());
    println!("  Hardware ID: {}", fingerprint.hardware_id);
    println!(
        "  Temperature at capture: {:.1}°C\n",
        fingerprint.temperature_celsius
    );

    // Validate against same hardware (should be valid)
    println!("Validating against same hardware...");
    let validation = fingerprint.validate(&mut hardware)?;
    print_validation(&validation);

    // Simulate small drift (thermal fluctuation)
    println!("\nSimulating small thermal drift (5%)...");
    hardware.drift_t_matrix(0.05);
    let validation = fingerprint.validate(&mut hardware)?;
    print_validation(&validation);

    // Simulate significant drift
    println!("\nSimulating significant drift (30%)...");
    hardware.drift_t_matrix(0.25); // Additional 25%
    let validation = fingerprint.validate(&mut hardware)?;
    print_validation(&validation);

    // Validate against different hardware
    println!("\nValidating against different hardware...");
    let mut different_hardware = MockOpticalHardware::new(999);
    println!("  Different hardware ID: {}", different_hardware.id());
    let validation = fingerprint.validate(&mut different_hardware)?;
    print_validation(&validation);

    // Demonstrate fingerprint comparison helpers
    println!("\n--- Fingerprint Validation Helpers ---");

    let validations = [
        ("Same hardware", FingerprintValidation::Valid),
        (
            "Drifted hardware",
            FingerprintValidation::Drifted {
                correlation: 0.85,
                estimated_drift: 0.15,
            },
        ),
        (
            "Different hardware",
            FingerprintValidation::DifferentHardware {
                expected_id: "mock-a".to_string(),
                actual_id: "mock-b".to_string(),
            },
        ),
    ];

    for (name, val) in &validations {
        println!(
            "  {}: usable={}, needs_calibration={}",
            name,
            val.is_usable(),
            val.needs_full_calibration()
        );
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

fn print_validation(validation: &FingerprintValidation) {
    match validation {
        FingerprintValidation::Valid => {
            println!("  Result: VALID - Hardware matches fingerprint");
        }
        FingerprintValidation::Drifted {
            correlation,
            estimated_drift,
        } => {
            println!("  Result: DRIFTED - Hardware has changed slightly");
            println!("    Correlation: {:.3}", correlation);
            println!("    Estimated drift: {:.3}", estimated_drift);
            println!("    Recommendation: Quick recalibration may help");
        }
        FingerprintValidation::DifferentHardware {
            expected_id,
            actual_id,
        } => {
            println!("  Result: DIFFERENT HARDWARE - Not the same device");
            println!("    Expected ID: {}", expected_id);
            println!("    Actual ID: {}", actual_id);
            println!("    Recommendation: Full recalibration required");
        }
        FingerprintValidation::NoFingerprint => {
            println!("  Result: NO FINGERPRINT - No baseline to compare");
        }
    }
}
