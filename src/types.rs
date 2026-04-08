//! Type definitions for fault injection.

use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Operations that can be failed via injection.
///
/// These represent the core operational boundaries of standard internet-scale
/// infrastructure that require testing for resilience.
pub enum Operation {
    /// Memory mapping operations (e.g. `mmap`).
    Mmap,
    /// Reading from streams, file descriptors, or sockets.
    Read,
    /// Writing to streams, file descriptors, or sockets.
    Write,
    /// Memory allocation routines.
    Alloc,
    /// Sending data over concurrency channels or networks.
    Send,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
/// Fault types that can be injected.
///
/// The `fail_after` parameter specifies the number of *successful* calls
/// before the failure occurs. For example, `fail_after: 0` fails the very first call.
pub enum Fault {
    /// Fail the Nth mmap call.
    Mmap {
        /// Number of successful calls before the failure.
        fail_after: u64,
    },
    /// Fail the Nth read call.
    Read {
        /// Number of successful calls before the failure.
        fail_after: u64,
    },
    /// Fail the Nth write call.
    Write {
        /// Number of successful calls before the failure.
        fail_after: u64,
    },
    /// Fail the Nth allocation.
    Alloc {
        /// Number of successful calls before the failure.
        fail_after: u64,
    },
    /// Fail the Nth channel send.
    Send {
        /// Number of successful calls before the failure.
        fail_after: u64,
    },
    /// Fail probabilistically for a specific operation.
    Probabilistic {
        /// Operation to target.
        op: Operation,
        /// Probability of failure (0.0 to 1.0).
        probability: f64,
    },
    /// Fail continuously after N successful calls for a specific operation.
    Persistent {
        /// Operation to target.
        op: Operation,
        /// Number of successful calls before failing continuously.
        fail_after: u64,
    },
    /// Fail on multiple specific call counts for a specific operation.
    Multiple {
        /// Operation to target.
        op: Operation,
        /// Specific call counts to fail on.
        fail_points: Vec<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Error when injecting a fault.
pub enum InjectionError {
    /// A duplicate fail point was provided, meaning the operation is already
    /// scheduled to fail at that exact call count.
    DuplicateFailPoint,
}

impl core::fmt::Display for InjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DuplicateFailPoint => write!(
                f,
                "Duplicate fail point provided. Fix: Use unique fail points or clear existing faults."
            ),
        }
    }
}

impl core::error::Error for InjectionError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Summary of faults cleared by a `clear` operation.
///
/// Useful for asserting that exactly the expected number of faults were
/// remaining at the end of a test run.
pub struct ClearedFaults {
    /// Number of remaining mmap fail points cleared.
    pub mmap: usize,
    /// Number of remaining read fail points cleared.
    pub read: usize,
    /// Number of remaining write fail points cleared.
    pub write: usize,
    /// Number of remaining alloc fail points cleared.
    pub alloc: usize,
    /// Number of remaining send fail points cleared.
    pub send: usize,
}
