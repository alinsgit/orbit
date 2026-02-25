#!/bin/bash
# Orbit version bump script
# Usage: ./scripts/bump-version.sh 1.3.0

set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 1.3.0"
  exit 1
fi

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: Invalid version format. Expected X.Y.Z (e.g., 1.3.0)"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Bumping version to $VERSION..."

# 1. core/Cargo.toml — first version = line only
sed -i "0,/^version = \".*\"/s//version = \"$VERSION\"/" "$ROOT_DIR/core/Cargo.toml"
echo "  Updated core/Cargo.toml"

# 2. package.json
sed -i "s/\"version\": \"[0-9]*\.[0-9]*\.[0-9]*\"/\"version\": \"$VERSION\"/" "$ROOT_DIR/package.json"
echo "  Updated package.json"

# 3. core/tauri.conf.json
sed -i "s/\"version\": \"[0-9]*\.[0-9]*\.[0-9]*\"/\"version\": \"$VERSION\"/" "$ROOT_DIR/core/tauri.conf.json"
echo "  Updated core/tauri.conf.json"

echo ""
echo "Version bumped to $VERSION in all 3 files."
echo "Next steps:"
echo "  git add core/Cargo.toml package.json core/tauri.conf.json"
echo "  git commit -m \"bump: v$VERSION\""
echo "  git tag v$VERSION"
