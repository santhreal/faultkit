//! Fault injection API functions.

use crate::config::{GlobalState, OpState, ENABLED, STATE};
use crate::types::{ClearedFaults, Fault, InjectionError, Operation};
use alloc::vec;

/// Check if fault injection is enabled globally.
///
/// # Examples
///
/// ```rust
/// use faultkit::{inject, clear, is_enabled, Fault};
///
/// clear();
/// assert!(!is_enabled());
/// inject(Fault::Mmap { fail_after: 0 });
/// assert!(is_enabled());
/// clear();
/// ```
#[inline]
#[must_use]
pub fn is_enabled() -> bool {
    ENABLED.load(core::sync::atomic::Ordering::Relaxed)
}

/// Inject a fault. Appends to existing fail points if the fault type allows multiple.
///
///
/// # Errors
/// Returns `Err` if a duplicate fail point is specified.
///
/// The fault remains active until [`clear`] is called.
pub fn try_inject(fault: Fault) -> Result<(), InjectionError> {
    ENABLED.store(true, core::sync::atomic::Ordering::Relaxed);
    let mut state = STATE.lock();

    let (op, mut new_points, prob, persist) = match fault {
        Fault::Mmap { fail_after } => (Operation::Mmap, vec![fail_after], None, None),
        Fault::Read { fail_after } => (Operation::Read, vec![fail_after], None, None),
        Fault::Write { fail_after } => (Operation::Write, vec![fail_after], None, None),
        Fault::Alloc { fail_after } => (Operation::Alloc, vec![fail_after], None, None),
        Fault::Send { fail_after } => (Operation::Send, vec![fail_after], None, None),
        Fault::Probabilistic { op, probability } => (op, vec![], Some(probability), None),
        Fault::Persistent { op, fail_after } => (op, vec![], None, Some(fail_after)),
        Fault::Multiple { op, fail_points } => (op, fail_points, None, None),
    };

    let op_state = state.get_mut(op);

    for p in new_points.drain(..) {
        if op_state.fail_points.contains(&p) {
            return Err(InjectionError::DuplicateFailPoint);
        }
        op_state.fail_points.push(p);
    }
    if let Some(p) = prob {
        op_state.probability = p;
    }
    if let Some(p) = persist {
        op_state.persist_after = Some(p);
    }

    Ok(())
}

/// Inject a fault.
///
/// For strict error handling and clarity, this behaves identically to [`try_inject`].
///
/// # Errors
/// Returns an error if the injection fails (e.g. duplicate fail points).
pub fn inject(fault: Fault) -> Result<(), InjectionError> {
    try_inject(fault)
}

/// Clear all injected faults and return what was cleared.
pub fn clear() -> ClearedFaults {
    ENABLED.store(false, core::sync::atomic::Ordering::Relaxed);
    let mut state = STATE.lock();

    let cleared = ClearedFaults {
        mmap: state.mmap.fail_points.len(),
        read: state.read.fail_points.len(),
        write: state.write.fail_points.len(),
        alloc: state.alloc.fail_points.len(),
        send: state.send.fail_points.len(),
    };

    *state = GlobalState {
        mmap: OpState::new(),
        read: OpState::new(),
        write: OpState::new(),
        alloc: OpState::new(),
        send: OpState::new(),
    };

    cleared
}

/// Check if an mmap call should fail. Call this at instrumented mmap sites.
///
/// Returns `true` if the fault should be triggered.
///
/// # Examples
///
/// ```rust
/// use faultkit::{inject, clear, should_fail_mmap, Fault};
///
/// clear();
/// assert!(!should_fail_mmap());
/// inject(Fault::Mmap { fail_after: 0 });
/// assert!(should_fail_mmap());
/// clear();
/// ```
#[inline]
pub fn should_fail_mmap() -> bool {
    if !is_enabled() {
        return false;
    }
    STATE.lock().get_mut(Operation::Mmap).check()
}

/// Check if a read call should fail.
///
/// # Examples
///
/// ```rust
/// use faultkit::{inject, clear, should_fail_read, Fault};
///
/// clear();
/// assert!(!should_fail_read());
/// inject(Fault::Read { fail_after: 0 });
/// assert!(should_fail_read());
/// clear();
/// ```
#[inline]
pub fn should_fail_read() -> bool {
    if !is_enabled() {
        return false;
    }
    STATE.lock().get_mut(Operation::Read).check()
}

/// Check if a write call should fail.
///
/// # Examples
///
/// ```rust
/// use faultkit::{inject, clear, should_fail_write, Fault};
///
/// clear();
/// assert!(!should_fail_write());
/// inject(Fault::Write { fail_after: 0 });
/// assert!(should_fail_write());
/// clear();
/// ```
#[inline]
pub fn should_fail_write() -> bool {
    if !is_enabled() {
        return false;
    }
    STATE.lock().get_mut(Operation::Write).check()
}

/// Check if an allocation should fail.
///
/// # Examples
///
/// ```rust
/// use faultkit::{inject, clear, should_fail_alloc, Fault};
///
/// clear();
/// assert!(!should_fail_alloc());
/// inject(Fault::Alloc { fail_after: 0 });
/// assert!(should_fail_alloc());
/// clear();
/// ```
#[inline]
pub fn should_fail_alloc() -> bool {
    if !is_enabled() {
        return false;
    }
    STATE.lock().get_mut(Operation::Alloc).check()
}

/// Check if a channel send should fail.
///
/// # Examples
///
/// ```rust
/// use faultkit::{inject, clear, should_fail_send, Fault};
///
/// clear();
/// assert!(!should_fail_send());
/// inject(Fault::Send { fail_after: 0 });
/// assert!(should_fail_send());
/// clear();
/// ```
#[inline]
pub fn should_fail_send() -> bool {
    if !is_enabled() {
        return false;
    }
    STATE.lock().get_mut(Operation::Send).check()
}
