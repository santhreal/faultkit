# faultkit

Internet-scale fault injection for testing error paths, fail-after sequences, and edge cases natively without cumbersome mocks.

Inspired by SQLite's OOM and I/O error injection architecture, `faultkit` allows you to fail the Nth operation (allocation, read, write, mmap, channel send) deterministically or probabilistically, ensuring systems handle failures gracefully.

## Usage

```rust
use faultkit::{inject, inject_scoped, should_fail_alloc, should_fail_write, clear, Fault};

// Thread-scoped injection: fail the 3rd allocation (fail_after: 2 means after 2 successes)
inject(Fault::Alloc { fail_after: 2 }).unwrap();

if should_fail_alloc() {
    // Return simulated allocation error
}

clear();

// RAII scoped injection
{
    let _guard = inject_scoped(Fault::Write { fail_after: 0 }).unwrap();
    if should_fail_write() {
        // Handle simulated I/O error
    }
} // Fault cleared automatically on drop
```

## Features

- **Thread-Local Isolation**: Defaults to thread-scoped faults so parallel unit tests never poison each other.
- **Cross-Thread Injection**: Opt-in `inject_global` / `inject_scoped_global` for worker-pool or multi-thread fault testing.
- **RAII Guards**: `inject_scoped` and `inject_scoped_global` automatically restore previous fault state on drop.
- **Zero-Cost Hot Path**: Atomically short-circuited when inactive; zero overhead when unused.
- **`no_std` Support**: Usable in core/alloc environments with zero external dependencies.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
