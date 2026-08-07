All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-02

### Changed
- Fault-injection state is now thread-local. Under 0.1.1 the state was process-global, so `clear()` and guard drops in one test thread wiped or leaked injections into sibling threads and made consumer test suites flaky under parallel runs (observed in rulekit: ~20% failure rate plus lock poisoning). Thread-local state makes parallel fault-injection tests deterministic.

## [0.2.1] - 2026-08-07
### Fixed
- Fixed operation cross-masking in thread-local fault checks: `should_fail_*` now checks if the thread-local state for that specific operation is active rather than any operation, preventing thread-local faults on one operation (e.g. `Alloc`) from masking global faults on another operation (e.g. `Mmap`).

### Changed
- Standardized `Cargo.toml` author metadata to `Santh <64453045+santhreal@users.noreply.github.com>`.
- Declared `package.metadata.santh.status = "beta"`.
- Fixed `README.md` to accurately document the public `faultkit` API (`inject`, `inject_scoped`, `Fault`, etc.).

## [0.1.0] - 2026-04-08

### Added

- Initial release of `faultkit`.
- Fault injection primitives for testing error paths.
- Configurable fail counters and trigger conditions.
- Zero-cost abstractions when fault injection is disabled.
