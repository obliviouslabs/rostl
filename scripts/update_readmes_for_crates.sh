#!/bin/bash
# This script updates all the README files for publishing to crates.io, without breaking the visualization on Github.
# The key idea is each crate has a README.md and the generated README for crates.io is going to be README.crates.md, which is the concatenation of the crates README.md with the top level README.md.

TOP_README="README.md"
CRATES_DIR="crates"
CRATE_README="README.md"
CRATE_README_TARGET="README.crate.md"
VERSION=$(grep -E '^version\s*=' Cargo.toml | sed 's/^version\s*=\s*"\(.*\)"/\1/')

# Get the first 8 lines of the top-level README

cp .gitignore .gitignore.bak
sed -i '/README\.crate\.md/d' .gitignore
head -n 8 "$TOP_README" > /tmp/top_readme_head.txt
sed -i '/unit\.yml/d' /tmp/top_readme_head.txt

for crate in "$CRATES_DIR"/*; do
  if [[ -d "$crate" && -f "$crate/$CRATE_README" && -f "$crate/Cargo.toml" ]]; then
    name=$(tomlq -r '.package.name' "$crate/Cargo.toml")

    head -n 1 "$crate/$CRATE_README" > "$crate/$CRATE_README_TARGET"
    
    echo "[![Crates.io](https://img.shields.io/crates/v/$name.svg)](https://crates.io/crates/$name)
[![Docs](https://docs.rs/$name/badge.svg)](https://docs.rs/$name)
[![codecov](https://codecov.io/gh/obliviouslabs/rostl/graph/badge.svg?token=P4O03Z6M5X)](https://codecov.io/gh/obliviouslabs/rostl)" >> "$crate/$CRATE_README_TARGET"
    
    tail -n +2 "$crate/$CRATE_README" >> "$crate/$CRATE_README_TARGET"
    echo "" >> "$crate/$CRATE_README_TARGET"
    
    cat /tmp/top_readme_head.txt >> "$crate/$CRATE_README_TARGET"

    git add "$crate/$CRATE_README_TARGET"
  fi
done

mv .gitignore.bak .gitignore

rm -f /tmp/top_readme_head.txt
