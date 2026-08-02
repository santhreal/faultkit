# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-02

### Changed
- Fault-injection state is now thread-local. Under 0.1.1 the state was process-global, so `clear()` and guard drops in one test thread wiped or leaked injections into sibling threads and made consumer test suites flaky under parallel runs (observed in rulekit: ~20% failure rate plus lock poisoning). Thread-local state makes parallel fault-injection tests deterministic.

## [Unreleased]

## [0.1.0] - 2026-04-08

### Added

- Initial release of `faultkit`.
- Fault injection primitives for testing error paths.
- Configurable fail counters and trigger conditions.
- Zero-cost abstractions when fault injection is disabled.
