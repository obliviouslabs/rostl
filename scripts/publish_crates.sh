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

# Update the changelogs
cargo changelog --write $CRATES

echo "Please review the generated changelogs and README files before proceeding. Make sure you bump the version in the changelogs and README files."
read -p "Press Enter to continue or Ctrl+C to cancel..."

git add ./crates/*/CHANGELOG.md
git commit -m "Prepare release $VERSION: update CHANGELOG files"
git tag "v$VERSION"
git push origin main --tags

cargo make precommit
cargo doc --workspace --lib --examples --all-features --locked --no-deps
cargo smart-release $CRATES 
read -p "Press Enter to continue or Ctrl+C to cancel..."

cargo workspace publish --dry-run
echo "Ok, now run cargo workspace publish as many times as necessary to update all crates. (There is no tool that supports publishing dependencies in order, so just publishing multiple times until all crates are at the latest version is the way to go.)"

read -p "Press Enter to continue or Ctrl+C to cancel..."

