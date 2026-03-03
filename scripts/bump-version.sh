#!/usr/bin/env bash
# Orbit version bump script — cross-platform (macOS / Linux / Git Bash on Windows)
# Usage:
#   ./scripts/bump-version.sh 1.5.0            # update files only
#   ./scripts/bump-version.sh 1.5.0 --commit   # update + git commit + tag

set -euo pipefail

# ── Arguments ────────────────────────────────────────────────────────
VERSION="${1:-}"
AUTO_COMMIT="${2:-}"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version> [--commit]"
  echo "Example: $0 1.5.0"
  echo "         $0 1.5.0 --commit"
  exit 1
fi

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: Invalid version format. Expected X.Y.Z (e.g., 1.5.0)"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# ── Perl availability check ───────────────────────────────────────────
if ! command -v perl &>/dev/null; then
  echo "Error: perl is required but not found."
  echo "  macOS:  comes pre-installed"
  echo "  Linux:  sudo apt install perl"
  echo "  Windows: install via Git Bash or Strawberry Perl"
  exit 1
fi

echo "Bumping version to $VERSION..."
echo ""

# ── Helper: update a file and verify ─────────────────────────────────
update_file() {
  local label="$1"
  local file="$2"
  local perl_expr="$3"
  local verify_pattern="$4"

  if [ ! -f "$file" ]; then
    echo "  ✗ $label — file not found: $file"
    exit 1
  fi

  perl -i -pe "$perl_expr" "$file"

  if grep -qF "$verify_pattern" "$file"; then
    echo "  ✓ $label"
  else
    echo "  ✗ $label — update failed (pattern not found after edit)"
    echo "    Expected: $verify_pattern"
    exit 1
  fi
}

# ── 1. core/Cargo.toml ───────────────────────────────────────────────
# Only replace the FIRST occurrence of `version = "..."` (package version,
# not dependency versions). Uses a $done flag via perl state variable.
update_file \
  "core/Cargo.toml" \
  "$ROOT_DIR/core/Cargo.toml" \
  'if (!$done && s/^version = "[0-9]+\.[0-9]+\.[0-9]+"$/version = "'"$VERSION"'"/) { $done = 1 }' \
  "version = \"$VERSION\""

# ── 2. package.json ──────────────────────────────────────────────────
update_file \
  "package.json" \
  "$ROOT_DIR/package.json" \
  's/"version": "[0-9]+\.[0-9]+\.[0-9]+"/"version": "'"$VERSION"'"/' \
  "\"version\": \"$VERSION\""

# ── 3. core/tauri.conf.json ──────────────────────────────────────────
update_file \
  "core/tauri.conf.json" \
  "$ROOT_DIR/core/tauri.conf.json" \
  's/"version": "[0-9]+\.[0-9]+\.[0-9]+"/"version": "'"$VERSION"'"/' \
  "\"version\": \"$VERSION\""

echo ""
echo "All 3 files updated to v$VERSION."
echo ""

# ── Git commit & tag (optional) ───────────────────────────────────────
if [ "$AUTO_COMMIT" = "--commit" ]; then
  cd "$ROOT_DIR"
  git add core/Cargo.toml package.json core/tauri.conf.json
  git commit -m "bump: v$VERSION"
  git tag "v$VERSION"
  echo "Committed and tagged v$VERSION"
  echo ""
  echo "Next: git push && git push --tags"
else
  echo "Next steps:"
  echo "  git add core/Cargo.toml package.json core/tauri.conf.json"
  echo "  git commit -m \"bump: v$VERSION\""
  echo "  git tag v$VERSION"
  echo "  git push && git push --tags"
  echo ""
  echo "  Or run with --commit to do this automatically:"
  echo "  ./scripts/bump-version.sh $VERSION --commit"
fi
