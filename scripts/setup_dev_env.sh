# Install required rust packages:
cargo install cargo-workspace
cargo install cargo-criterion
cargo install cargo-hack
cargo install cargo-nextest
cargo install zepter

echo "cargo make precommit" > .git/hooks/pre-push
chmod +x .git/hooks/pre-push