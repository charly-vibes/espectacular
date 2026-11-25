#!/usr/bin/env bash
# Purpose: Release-freshness oracle for ship-close gates.
# Responsibilities: read-only checks — newest CHANGELOG.md release heading must
#   equal the Cargo.toml version, and docs/ must have no uncommitted changes.
#   Exit 0 = pass, exit 1 = fail; failure reason is written to stderr.
# Rationale: release/docs freshness checks were historically done manually;
#   wiring them as a pipeline oracle turns them into a verified milestone gate.
#   Accepts an artifact path as $1 (oracle contract) but inspects the repo.

set -euo pipefail

# Artifact path ($1) accepted per the oracle contract but unused: this oracle
# inspects repository state at gate time rather than artifact contents.

if [[ ! -f Cargo.toml ]]; then
  echo "release-docs-fresh: Cargo.toml not found in $(pwd)" >&2
  exit 1
fi
if [[ ! -f CHANGELOG.md ]]; then
  echo "release-docs-fresh: CHANGELOG.md not found in $(pwd)" >&2
  exit 1
fi

cargo_version="$(sed -nE 's/^version[[:space:]]*=[[:space:]]*\"([^\"]+)\".*/\1/p' Cargo.toml | head -1)"
if [[ -z "$cargo_version" ]]; then
  echo "release-docs-fresh: no package version found in Cargo.toml" >&2
  exit 1
fi

# Newest non-Unreleased release heading: `## [X] - date`
# `|| true` guards against set -euo pipefail aborting on grep no-match (which
# would skip the custom 'no release heading' error below).
changelog_version="$(grep -E '^## \[' CHANGELOG.md | grep -v '\[Unreleased\]' | head -1 | sed -E 's/^## \[([^]]+)\].*/\1/' || true)"
if [[ -z "$changelog_version" ]]; then
  echo "release-docs-fresh: CHANGELOG.md has no release heading (only Unreleased); Cargo.toml version is $cargo_version" >&2
  exit 1
fi

if [[ "$changelog_version" != "$cargo_version" ]]; then
  echo "release-docs-fresh: CHANGELOG newest release ($changelog_version) lags Cargo.toml version ($cargo_version)" >&2
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "release-docs-fresh: not a git repository; cannot verify docs/ freshness" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain -- docs/ 2>/dev/null)" ]]; then
  echo "release-docs-fresh: docs/ has uncommitted changes at gate time — commit or clean docs/ before release" >&2
  exit 1
fi

exit 0
