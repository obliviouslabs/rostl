#!/bin/bash
set -e

# Get crates from cargo.toml using jq
CRATES=""
for path in $(tomlq -r '.workspace.members[]' Cargo.toml); do
  name=$(tomlq -r '.package.name' "$path/Cargo.toml")
  CRATES+="$name "
done

# Trim trailing space
CRATES=$(echo "$CRATES" | xargs)
echo "Crates to be published: $CRATES"

# Check if there are no uncommitted or unpushed changes
if ! git diff-index --quiet HEAD --; then
    echo "There are uncommitted changes. Please commit or stash them before publishing."
    exit 1
fi
if ! git diff-index --quiet HEAD@{upstream} --; then
    echo "There are unpushed changes. Please push them before publishing."
    exit 1
fi
# Check if the current branch is main
if [ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]; then
    echo "You must be on the main branch to publish crates."
    exit 1
fi
# Check if the current branch is clean
if ! git diff-index --quiet HEAD --; then
    echo "The working directory is not clean. Please commit or stash your changes."
    exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep -E '^version\s*=' Cargo.toml | sed 's/^version\s*=\s*"\(.*\)"/\1/')
if [ -z "$VERSION" ]; then
    echo "Could not find version in Cargo.toml"
    exit 1
fi

cargo make precommit
cargo outdated
echo "Publishing crates with version $VERSION..."

cargo smart-release $CRATES 
read -p "Press Enter to continue or Ctrl+C to cancel..."

cargo make precommit
cargo doc --workspace --lib --examples --all-features --locked --no-deps
cargo smart-release --execute --no-publish $CRATES 
git tag "v$VERSION"

echo "Final check before publishing: "
read -p "Press Enter to continue or Ctrl+C to cancel..."

for _ in `seq 1 5`; do
    cargo workspace publish
done
