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


# TDD loop (most obvious entry, DDL test-runner standard — see DDL-85p in
# the dulce-de-leche repo): testaruda exec — select the tests the working
# diff affects, run only those, ingest, calibrate. Tolerance contract:
# exit 0/10/20 green; NOT CALIBRATED with 0 actual failures is a vacuous
# pass (tambor-cpd policy), with failures it blocks. Local store
# (.testaruda/, gitignored) initializes on first use; a stale store
# self-heals (wild-z56).
test:
    #!/usr/bin/env bash
    set -uo pipefail
    if [ ! -d .testaruda ]; then
      testaruda init || { echo "❌ testaruda init failed" >&2; exit 1; }
    fi
    code=0
    testaruda exec || code=$?
    case "$code" in
      0|10|20) exit 0 ;;
      4)
        echo "⚠️ runner exit 4 — pruning stale local calibration store and retrying once (wild-z56)" >&2
        rm -rf .testaruda
        testaruda init || { echo "❌ testaruda re-init failed" >&2; exit 1; }
        testaruda exec ;;
      *) exit "$code" ;;
    esac

# Blast-radius preview: which tests a change WOULD affect — runs nothing.
# Useful before a refactor; optionally name files: just test-blast src/foo.rs
test-blast *files:
    #!/usr/bin/env bash
    if [ -z "{{files}}" ]; then
      testaruda select --pre-edit --human
    else
      testaruda select --pre-edit --human --files "{{files}}"
    fi

test-full:
    cargo test

# Lint with clippy (--all-targets: test-code lints only fail here otherwise
# they slip through to the pre-push hook)
lint:
    cargo clippy --all-targets -- -D warnings

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

