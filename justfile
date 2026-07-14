default:
    @just --list

# Install ah binary to ~/.cargo/bin
install:
    cargo install --path . --locked

prime:
    wai prime || wai status
    bd prime
    openspec list || true
    dont prime --plain || true

status:
    wai status
    bd ready
    openspec list || true
    dont list --plain || true

validate:
    openspec validate --all

plugins:
    wai plugin list

# === Rust Commands ===

# Build release binary
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Lint with clippy
lint:
    cargo clippy -- -D warnings

# Check formatting
fmt-check:
    cargo fmt -- --check

# === CI Commands ===

# Full CI pipeline (matches the CI workflow)
ci: fmt-check lint test build-release