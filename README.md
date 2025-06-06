# rods: Rust Oblivious Data Structures

[![Crates.io](https://img.shields.io/crates/v/TODO.svg)](https://crates.io/crates/TODO)
[![Docs](https://docs.rs/TODO/badge.svg)](https://docs.rs/TODO)
[![CI](https://github.com/xtrm0/rods/actions/workflows/bench.yml/badge.svg)](https://github.com/xtrm0/rods/actions/workflows/bench.yml)

**rods** (Rust Oblivious Data Structures) is a Rust library providing a suite of high-performance, data- and instruction-trace oblivious data structures and algorithms, designed for use in Trusted Execution Environments (TEEs) such as Intel TDX. All memory accesses and instructions executed are independent of the data being processed, providing strong security guarantees against side-channel attacks.

## Why Oblivious Data Structures?

In TEEs, attackers may observe memory access patterns or instruction traces, even if the data itself is encrypted. Traditional data structures can leak sensitive information through these side channels. Oblivious data structures and algorithms ensure that both of the following do not depend on secret data being processed:

- **Memory access patterns** 
- **Instruction traces** 

This is critical for applications in confidential computing, privacy-preserving analytics, secure enclaves, and anywhere side-channel resistance is required.

## Features

- **Oblivious Arrays**: Fixed-size and dynamic - access patterns do not leak which indices are being accessed.
- **Oblivious Maps**: Cuckoo-hash-based and sharded maps with batch and single-key APIs.
- **Oblivious Heaps & Priority Queues**: Oblivious heap implementations.
- **Oblivious Stacks & Queues**: Data-independent push/pop operations.
- **Oblivious Vectors**: Variable-length vectors with oblivious access.
- **Oblivious Sorting & Permutation**: Bitonic, Batcher, and Bose-Nelson sorters, compaction, and shuffling.
- **Oblivious RAM (ORAM)**: Circuit ORAM, Linear ORAM, and Recursive ORAM implementations.
- **Primitives**: Constant-time conditional move/swap traits, indexable abstractions, and utility functions.
- **External Memory Abstractions**: For scalable, oblivious storage.

All data structures are designed to be used in TEE environments and are implemented with rigorous attention to side-channel resistance.

## Project Structure

- `crates/datastructures`: Core oblivious data structures (arrays, maps, heaps, stacks, queues, vectors, sharded maps).
- `crates/oram`: Oblivious RAM algorithms (Circuit ORAM, Linear ORAM, Recursive ORAM, HeapTree).
- `crates/primitives`: Constant-time primitives, traits, and utilities.
- `crates/sort`: Oblivious sorting, compaction, and permutation algorithms.
- `crates/storage`: External memory abstractions for oblivious storage.
- `scripts/`: Developer scripts for benchmarking, code quality, and automation.

## Usage

Add the relevant crate(s) to your `Cargo.toml`:

```toml
[dependencies]
rods-datastructures = "1.0"
```

Example: Creating and using an oblivious array

```rust
use rods_datastructures::array::LongArray;

let mut arr = LongArray::<u64, 1024>::new();
arr.write(42, 1234);
let mut value = 0;
arr.read(42, &mut value);
assert_eq!(value, 1234);
```

All APIs are designed to be as close as possible to their standard Rust counterparts, but with obliviousness guarantees.

## Documentation

- **API Docs**: [docs.rs/TODO](https://docs.rs/TODO)
- **Generated Documentation**: Run `cargo doc --workspace --lib --all-features --no-deps` and open the output in your browser.

## Benchmarks & Performance

- **Automated Benchmarks**: Benchmarks are run on every commit via GitHub Actions. See `.github/workflows/bench.yml`.
- **Performance Regression Checks**: Benchmarks are compared against rules in `scripts/benchmark_rules.txt` to catch regressions.
- **Comparisons**: For cross-project benchmarks, see the companion benchmarking repository (link in the main repo).

## Code Quality

- **Strict Linting**: All code is checked with `clippy`, `cargo fmt`, and custom lints.
- **Pre-commit & Pre-merge Checks**: Automated via `Makefile.toml` and GitHub Actions.
- **Dependency Auditing**: Uses `cargo-deny` for dependency and license checks.

## Testing

- **Comprehensive Tests**: All data structures and algorithms are covered by unit and property-based tests.
- **How to Run**: `cargo test --workspace`

## Contributing

We welcome contributions! If you are interested in:

- Implementing new oblivious data structures or algorithms
- Improving performance or security
- Adding new TEE backends or features
- Writing documentation or tutorials

Please open an issue or pull request. See the code comments and module-level docs for guidance. All contributions must pass code quality and benchmark checks.

## Research & References

- [Circuit ORAM](https://eprint.iacr.org/2014/672.pdf)
- [Path Oblivious Heap](https://eprint.iacr.org/2019/274)
- [Intel TDX](https://www.intel.com/content/www/us/en/architecture-and-technology/tdx.html)

## License

Licensed under MIT or Apache-2.0, at your option.

---

**rods** aims to be the go-to library for building secure, high-performance, and side-channel-resistant applications in Rust. We invite you to use, extend, and contribute to the project!
