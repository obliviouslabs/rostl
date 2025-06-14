# rostl: Rust Oblivious Standard Library

[![Crates.io](https://img.shields.io/crates/v/TODO.svg)](https://crates.io/crates/TODO)
[![Docs](https://docs.rs/TODO/badge.svg)](https://docs.rs/TODO)
[![CI](https://github.com/obliviouslabs/rostl/actions/workflows/unit.yml/badge.svg)](https://github.com/obliviouslabs/rostl/actions/workflows/unit.yml)
[![codecov](https://codecov.io/gh/obliviouslabs/rostl/graph/badge.svg?token=L26XUTDO79)](https://codecov.io/gh/obliviouslabs/rostl)

**rostl** (Rust Oblivious Standard Library) is a Rust library providing a suite of high-performance, data- and instruction-trace oblivious data structures and algorithms, designed for use in Trusted Execution Environments (TEEs) such as Intel TDX. All memory accesses and instructions executed are independent of the data being processed, providing strong security guarantees against side-channel attacks.
# rostl-primitives

This crate provides constant-time primitives, traits, and utility functions for building oblivious algorithms and data structures. Includes conditional move/swap, indexable abstractions, and more.

See the main project [README](https://github.com/obliviouslabs/rostl/README.md) for more details, usage, and contribution guidelines.
