# rostl-oram
[![Crates.io](https://img.shields.io/crates/v/rostl-oram.svg)](https://crates.io/crates/rostl-oram)
[![Docs](https://docs.rs/rostl-oram/badge.svg)](https://docs.rs/rostl-oram)
[![CI](https://github.com/obliviouslabs/rostl/actions/workflows/unit.yml/badge.svg)](https://github.com/obliviouslabs/rostl/actions/workflows/unit.yml)
[![codecov](https://codecov.io/gh/obliviouslabs/rostl/graph/badge.svg?token=L26XUTDO79)](https://codecov.io/gh/obliviouslabs/rostl)

This crate implements several Oblivious RAM (ORAM) algorithms, including Circuit ORAM, Linear ORAM, and Recursive ORAM, for use in TEEs and privacy-preserving systems. All memory accesses are data-independent.

See the main project [README](https://github.com/obliviouslabs/rostl/) for more details, usage, and contribution guidelines.

# rostl: Rust Oblivious Standard Library

[![Crates.io](https://img.shields.io/crates/v/0.1.0.svg)](https://crates.io/crates/rostl-datastructures)
[![Docs](https://docs.rs/rostl-datastructures/badge.svg)](https://docs.rs/rostl-datastructures)
[![CI](https://github.com/obliviouslabs/rostl/actions/workflows/unit.yml/badge.svg)](https://github.com/obliviouslabs/rostl/actions/workflows/unit.yml)
[![codecov](https://codecov.io/gh/obliviouslabs/rostl/graph/badge.svg?token=L26XUTDO79)](https://codecov.io/gh/obliviouslabs/rostl)

**rostl** (Rust Oblivious Standard Library) is a Rust library providing a suite of high-performance, data- and instruction-trace oblivious data structures and algorithms, designed for use in Trusted Execution Environments (TEEs) such as Intel TDX. All memory accesses and instructions executed are independent of the data being processed, providing strong security guarantees against side-channel attacks.
