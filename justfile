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

# === Spec Commands ===

# Specodelic dual-format gates over the openspec corpus (see
# openspec/changes/migrate-specs-to-dual-format): deep lint of dual-format
# files plus the frontmatter mandate over every non-archived spec.md.
spec-lint:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v spk >/dev/null || { echo "spk missing: cargo install specodelic"; exit 1; }
    spk lint openspec
    plain=$(find openspec/specs openspec/changes -name spec.md -not -path '*/archive/*' -exec bash -c '[ "$(head -1 "$1")" = "---" ] || echo "$1"' _ {} \;)
    if [ -n "$plain" ]; then
        echo "spec.md files missing specodelic frontmatter (migration backlog, see openspec/changes/migrate-specs-to-dual-format):"
        echo "$plain"
        exit 1
    fi
    echo "spec corpus: all spec.md files dual-format and lint-clean"

# === CI Commands ===

# Full CI pipeline (matches the CI workflow)
ci: fmt-check lint test build-release spec-lint

