#!/bin/bash
# This script updates all the README files for publishing to crates.io, without breaking the visualization on Github.
# The key idea is each crate has a README.md and the generated README for crates.io is going to be README.crates.md, which is the concatenation of the crates README.md with the top level README.md.

TOP_README="README.md"
CRATES_DIR="crates"
CRATE_README="README.md"
CRATE_README_TARGET="README.crate.md"

# Get the first 8 lines of the top-level README
head -n 8 "$TOP_README" > /tmp/top_readme_head.txt

for crate in "$CRATES_DIR"/*; do
  if [[ -d "$crate" && -f "$crate/$CRATE_README" && -f "$crate/Cargo.toml" ]]; then
    # Combine for README.crate.md
    cat /tmp/top_readme_head.txt "$crate/$CRATE_README_TARGET"

    # Append the local README.md to the local README.crate.md
    cat "$crate/$CRATE_README" >> "$crate/$CRATE_README_TARGET"

    # Replace cargo.toml README.md with README.crate.md
    sed -i "s|^readme = .*|readme = \"${CRATE_README_TARGET}\"|" "$crate/Cargo.toml"
  fi
done

rm -f /tmp/top_readme_head.txt
