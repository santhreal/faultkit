# faultkit

Fault injection for testing error paths. Fail the Nth syscall and verify graceful handling.

## Usage

```rust
use faultkit::FaultInjector;

let injector = FaultInjector::new();

// Fail the 3rd allocation
injector.set_fail_at(3);

// Your code runs normally for 2 allocations, then the 3rd fails
// Verify your code handles the failure gracefully
```

## Why

Testing error paths is hard. Most code has `unwrap()` or `?` on operations that "never fail" — until they do in production. faultkit lets you systematically test every error path by failing operations at specific points.

## License

MIT
