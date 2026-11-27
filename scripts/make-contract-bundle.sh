#!/usr/bin/env bash
# Purpose: produce the release contract bundle (rung-2 fleet rollout, beads
#   wild-39w, openspec change add-wild-contract-bundle-convention task 3.2):
#   run `wild extract` on the exact release checkout, stamp the digest-bound
#   manifest, and pack the two-file bundle for release attachment.
# Responsibilities: enumerate the tracked checkout (git ls-files) with raw
#   sha256 commitments; write the extraction request (profile
#   rust-cargo-local-1, fixed target/toolchain/features, extractor identity);
#   invoke the real wild binary (no fabricated contract, no retry); on exit 0
#   or 1 (accept, or honest refuse-Unknown gap accounting) stamp
#   `manifest.toml` {tool, version, commit, profile, contract_sha256,
#   extractor_version, extracted_at} and pack tar+gzip as
#   `ah-<version>-contract-bundle.tar.gz`; on exit 2 (error class:
#   malformed input, digest mismatch, out-of-bundle reference — no contract
#   exists to stamp) fail loudly.
# Rationale: the manifest is the digest-bound trust-chain head; anything
#   absent from it is unverifiable by construction. The bundle stays exactly
#   two files (contract.json + manifest.toml). Unknown verdicts are a
#   first-class outcome downstream — a gap-bearing bundle is still honest
#   evidence, so exit 1 attaches rather than blocks the release. Mirrors the
#   wild convention script byte-for-byte except the tool identity.
#
# Usage: make-contract-bundle.sh <version> <output-dir>
# Env:   WILD_BIN  — extractor binary (default `wild` on PATH)

set -euo pipefail

VERSION="${1:?usage: make-contract-bundle.sh <version> <output-dir>}"
OUT_DIR="${2:?usage: make-contract-bundle.sh <version> <output-dir>}"
WILD_BIN="${WILD_BIN:-wild}"
TOOL="ah"
PROFILE="rust-cargo-local-1"
TARGET="x86_64-unknown-linux-gnu"

cd "$(git rev-parse --show-toplevel)"
EXTRACTOR_VERSION="$("$WILD_BIN" --version)" || {
  echo "make-contract-bundle: $WILD_BIN --version failed" >&2
  exit 2
}
COMMIT="$(git rev-parse HEAD)"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Commitments over raw bytes of every tracked, present, non-symlink file.
python3 - "$PWD" "$WORK/extraction.json" "$TARGET" "$PROFILE" <<'PYEOF'
import hashlib, json, subprocess, sys
from pathlib import Path

root, request_path, target, profile = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], sys.argv[4]
listed = subprocess.run(
    ["git", "ls-files"], capture_output=True, text=True, cwd=root, check=True
).stdout.splitlines()
files = {}
for rel in listed:
    p = root / rel
    if p.is_file() and not p.is_symlink():
        files[rel] = "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()
request = {
    "version": "local-1",
    "kind": "extraction-request",
    "profile": profile,
    "bundle_root": str(root),
    "files": dict(sorted(files.items())),
    "target": target,
    "toolchain": "stable",
    "features": [],
    "extractor": {
        "name": "wild",
        "digest": "sha256:" + hashlib.sha256(b"wild-extractor-v0").hexdigest(),
    },
}
request_path.write_text(json.dumps(request, indent=1))
print(f"make-contract-bundle: committed {len(files)} tracked files")
PYEOF

BUNDLE_DIR="$WORK/contract-bundle"
set +e
"$WILD_BIN" extract --request "$WORK/extraction.json" --output "$BUNDLE_DIR"
RC=$?
set -e
echo "make-contract-bundle: wild extract exit=$RC"
if [ "$RC" -ne 0 ] && [ "$RC" -ne 1 ]; then
  echo "make-contract-bundle: error-class extraction (exit $RC) — no contract to stamp, refusing" >&2
  exit 2
fi

CONTRACT_SHA256="sha256:$(sha256sum "$BUNDLE_DIR/contract.json" | cut -d' ' -f1)"
cat > "$BUNDLE_DIR/manifest.toml" <<EOF
tool = "$TOOL"
version = "$VERSION"
commit = "$COMMIT"
profile = "$PROFILE"
contract_sha256 = "$CONTRACT_SHA256"
extractor_version = "$EXTRACTOR_VERSION"
extracted_at = "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
EOF

mkdir -p "$OUT_DIR"
ASSET="$OUT_DIR/${TOOL}-${VERSION}-contract-bundle.tar.gz"
tar -czf "$ASSET" -C "$BUNDLE_DIR" contract.json manifest.toml
echo "make-contract-bundle: wrote $ASSET (contract_sha256=$CONTRACT_SHA256)"
