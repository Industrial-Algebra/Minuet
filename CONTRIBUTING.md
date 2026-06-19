# Contributing to Minuet

Thank you for your interest in contributing! Minuet is an Industrial Algebra
project, dual-licensed under AGPL v3 and a commercial license.

## Contributor License Agreement (CLA)

Minuet is dual-licensed (AGPL v3 + commercial). To enable this model,
**all contributors must sign a Contributor License Agreement (CLA)**.

The CLA grants Industrial Algebra the right to relicense your contributions
under the commercial license, while you retain full copyright ownership of
your contributions. Without a CLA, your contributions can only be used under
AGPL v3 terms, which would prevent Industrial Algebra from offering a
commercial license for the combined work.

### How to Sign

1. Download the CLA from: https://industrial-algebra.org/cla
2. Sign and email to: cla@industrial-algebra.org
3. Include your GitHub username in the email

Pull requests from contributors who have not signed the CLA cannot be merged.

## Development Setup

```bash
# Clone and build
git clone https://github.com/industrial-algebra/Minuet
cd Minuet
cargo build
cargo test
cargo clippy --all-targets
```

## Conventions

- **Rust edition 2021**, nightly toolchain (default via `rust-toolchain.toml`)
- `#![warn(missing_docs)]` — every public item must be documented
- `#![warn(clippy::all)]` — zero clippy warnings
- Generic over `BindingAlgebra` from `amari-holographic`
- Default algebra for tests: `ProductCliffordAlgebra<8>` (64 dimensions)
- Feature gates are additive — never break existing API
- Optical tests require `--features optical`

## Pull Request Process

1. Sign the CLA (see above)
2. Ensure `cargo test --all-features` passes (or features excluding `persistence` if C++ toolchain unavailable)
3. Ensure `cargo clippy --all-targets` is clean (except `persistence` feature)
4. Ensure `cargo fmt` passes
5. Add tests for new functionality
6. Update documentation (module docs, README if applicable)
7. Add a CHANGELOG entry under an `[Unreleased]` section

## Testing

```bash
# Core tests (no features)
cargo test

# With optical backend
cargo test --features optical

# With all working features (exclude persistence if C++ unavailable)
cargo test --features "parallel,serde,async,optical"

# Clippy
cargo clippy --features "parallel,serde,async,optical"

# Format
cargo fmt --all --check
```

## License

By contributing, you agree that your contributions will be licensed under
the same dual-licensing model as the project (AGPL v3 + commercial).
