# Install required rust packages:
cargo install cargo-workspace
cargo install cargo-criterion
cargo install cargo-hack
cargo install cargo-nextest
cargo install zepter
cargo install cargo-make
cargo install cargo-show-asm
cargo install cargo-udeps
cargo install cargo-deny
cargo install cargo-llvm-cov

echo "cargo make precommit" > .git/hooks/pre-push
chmod +x .git/hooks/pre-push

# Check if Homebrew is installed, and install codespell if it is
if command -v brew &> /dev/null; then
    echo "Homebrew is installed. Installing codespell..."
    brew install codespell
else
    echo "Homebrew is not installed. Skipping codespell installation. You may need to manually install codespell to push"
fi

git config --local include.path ../.gitconfig
