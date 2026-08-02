# faultkit: Technical Spec

## Overview

`faultkit`: Internet-scale fault injection for testing complex error paths and edge cases.  Inspired by [SQLite's OOM and IO error injection](https://www.sqlite.org/testing.html#fault_injection), `faultkit` lets you fail the Nth call to a specific operation and verify the system handles it gracefully. It is designed for maximum performance, robustness at scale, and comprehensive test coverage.  # WHAT this crate does `faultkit` provides a zero-cost abstraction for injecting targeted failures (like allocation errors, I/O errors, or channel send failures) into your system's critical paths. It manages global, atomic state tracking fail conditions and triggers failures probabilistically, persistently, or on precise counts.  # WHY someone would use it Testing happy paths is trivial, but ensuring an internet-scale system gracefully handles an OOM or a disconnected network socket during a million-file write requires deterministic fault injection. `faultkit` forces these errors natively without requiring cumbersome stubs or mocks.  # HOW to get started ```rust use faultkit::{inject, clear, should_fail_mmap, Fault};  // Make the 3rd mmap call fail let _ = inject(Fault::Mmap { fail_after: 3 });  // ... in your code ... if should_fail_mmap() { // return simulated error }  // Clean up let _ = clear(); ```  # Compile-time control Fault injection is always available. The state check is completely zero-cost when not active, enabling the compiler to optimize the check away in hot paths.

## Architecture

The crate is organized into the following public modules:

- (see source for module layout)

## Guarantees

- `#![forbid(unsafe_code)]` where applicable; see `src/lib.rs` for the exact lint preamble.
- All public types have doc comments.
- Error messages are actionable where applicable.

## Public API Summary

Key entry points are exported from `src/lib.rs` via `pub mod` and `pub use` re-exports.
Consult the module-level documentation in each source file for function signatures and usage examples.

## Error Handling

- Standard `Result` / error types.
