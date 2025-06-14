# rostl-primitives
[![Crates.io](https://img.shields.io/crates/v/rostl-primitives.svg)](https://crates.io/crates/rostl-primitives)
[![Docs](https://docs.rs/rostl-primitives/badge.svg)](https://docs.rs/rostl-primitives)
[![codecov](https://codecov.io/gh/obliviouslabs/rostl/graph/badge.svg?token=P4O03Z6M5X)](https://codecov.io/gh/obliviouslabs/rostl)

This crate provides constant-time primitives, traits, and utility functions for building oblivious algorithms and data structures. Includes conditional move/swap, indexable abstractions, and more.

See the main project [README](https://github.com/obliviouslabs/rostl/) for more details, usage, and contribution guidelines.

# rostl: Rust Oblivious Standard Library

[![Crates.io](https://img.shields.io/crates/v/rostl-datastructures.svg)](https://crates.io/crates/rostl-datastructures)
[![Docs](https://docs.rs/rostl-datastructures/badge.svg)](https://docs.rs/rostl-datastructures)
[![codecov](https://codecov.io/gh/obliviouslabs/rostl/graph/badge.svg?token=P4O03Z6M5X)](https://codecov.io/gh/obliviouslabs/rostl)

**rostl** (Rust Oblivious Standard Library) is a Rust library providing a suite of high-performance, data- and instruction-trace oblivious data structures and algorithms, designed for use in Trusted Execution Environments (TEEs) such as Intel TDX. All memory accesses and instructions executed are independent of the data being processed, providing strong security guarantees against side-channel attacks.
