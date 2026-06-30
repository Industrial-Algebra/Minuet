# Contributing to Minuet

Thank you for your interest in contributing! Minuet is an Industrial Algebra
project, licensed under **Apache-2.0**.

## Contributor License Agreement (CLA)

All contributors must sign the [Contributor License Agreement
(CLA)](https://github.com/Industrial-Algebra/.github/blob/main/CLA.md). The
CLA grants Industrial Algebra a copyright license to your contributions and the
right to relicense them, while you retain full copyright ownership of your
work. It also provides a patent grant with a retaliation clause. Signing once
covers all Industrial Algebra projects.

### How to Sign

1. Read the CLA: https://github.com/Industrial-Algebra/.github/blob/main/CLA.md
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
**Apache-2.0**, and that you have signed the CLA above granting Industrial
Algebra the rights described therein.
