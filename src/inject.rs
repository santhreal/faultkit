//! Fault injection API functions.

use crate::config::{
    GlobalState, ENABLED, STATE, thread_local_active, thread_local_op_active,
    with_thread_local_state_mut,
};
use crate::types::{ClearedFaults, Fault, InjectionError, Operation};
use alloc::vec;

/// Check if fault injection is enabled for the current thread or globally.
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
    ENABLED.load(core::sync::atomic::Ordering::Relaxed) || thread_local_active()
}

/// Inject `fault` into the current thread's isolated fault state.
///
/// The fault only fires on the thread that injected it. This is the default
/// injection path and is safe to use from parallel tests.
///
/// # Errors
/// Returns `Err` if a duplicate fail point is specified.
///
/// The fault remains active until [`clear`] is called.
pub fn try_inject(fault: Fault) -> Result<(), InjectionError> {
    with_thread_local_state_mut(|state| inject_into_state(state, fault))
}

/// Inject `fault` globally so it is visible across all threads.
///
/// This is the explicit opt-in for cross-thread fault scenarios (e.g., a worker
/// pool that wants a fault to fire on any worker thread).
///
/// # Errors
/// Returns `Err` if a duplicate fail point is specified.
///
/// The fault remains active until [`clear`] is called.
pub fn try_inject_global(fault: Fault) -> Result<(), InjectionError> {
    let mut state = STATE.lock();
    inject_into_state(&mut *state, fault)?;
    let any = state.any_active();
    ENABLED.store(any, core::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// Shared implementation for thread-local and global injection.
fn inject_into_state(state: &mut GlobalState, fault: Fault) -> Result<(), InjectionError> {
    let (op, new_points, prob, persist) = match fault {
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

    // Validate ALL new points BEFORE mutating any state, so a duplicate (against
    // an existing point OR another point within this same batch) leaves the
    // state entirely untouched instead of half-injected. The BTreeSet also gives
    // O((existing + new) log n) membership in place of the old O(existing * new)
    // `contains()` scan per point.
    if !new_points.is_empty() {
        let mut seen: alloc::collections::BTreeSet<u64> =
            op_state.fail_points.iter().copied().collect();
        for &p in &new_points {
            if !seen.insert(p) {
                return Err(InjectionError::DuplicateFailPoint);
            }
        }
    }

    // Validation passed: apply atomically.
    op_state.fail_points.extend(new_points);
    if let Some(p) = prob {
        op_state.probability = p;
    }
    if let Some(p) = persist {
        op_state.persist_after = Some(p);
    }

    Ok(())
}

/// Inject a fault into the current thread.
///
/// For strict error handling and clarity, this behaves identically to [`try_inject`].
///
/// # Errors
/// Returns an error if the injection fails (e.g., duplicate fail points).
pub fn inject(fault: Fault) -> Result<(), InjectionError> {
    try_inject(fault)
}

/// Inject a fault globally.
///
/// For strict error handling and clarity, this behaves identically to [`try_inject_global`].
///
/// # Errors
/// Returns an error if the injection fails (e.g., duplicate fail points).
pub fn inject_global(fault: Fault) -> Result<(), InjectionError> {
    try_inject_global(fault)
}

/// Clear all injected faults and return what was cleared.
///
/// This clears both the current thread's isolated state and the process-global
/// state, then returns a combined summary.
pub fn clear() -> ClearedFaults {
    let mut global = STATE.lock();

    let global_cleared = ClearedFaults {
        mmap: global.mmap.fail_points.len(),
        read: global.read.fail_points.len(),
        write: global.write.fail_points.len(),
        alloc: global.alloc.fail_points.len(),
        send: global.send.fail_points.len(),
        persistent: usize::from(global.mmap.persist_after.is_some())
            + usize::from(global.read.persist_after.is_some())
            + usize::from(global.write.persist_after.is_some())
            + usize::from(global.alloc.persist_after.is_some())
            + usize::from(global.send.persist_after.is_some()),
        probabilistic: usize::from(global.mmap.probability > 0.0)
            + usize::from(global.read.probability > 0.0)
            + usize::from(global.write.probability > 0.0)
            + usize::from(global.alloc.probability > 0.0)
            + usize::from(global.send.probability > 0.0),
    };

    *global = GlobalState::new();
    ENABLED.store(false, core::sync::atomic::Ordering::Relaxed);
    drop(global);

    let local_cleared = with_thread_local_state_mut(|state| {
        let cleared = ClearedFaults {
            mmap: state.mmap.fail_points.len(),
            read: state.read.fail_points.len(),
            write: state.write.fail_points.len(),
            alloc: state.alloc.fail_points.len(),
            send: state.send.fail_points.len(),
            persistent: usize::from(state.mmap.persist_after.is_some())
                + usize::from(state.read.persist_after.is_some())
                + usize::from(state.write.persist_after.is_some())
                + usize::from(state.alloc.persist_after.is_some())
                + usize::from(state.send.persist_after.is_some()),
            probabilistic: usize::from(state.mmap.probability > 0.0)
                + usize::from(state.read.probability > 0.0)
                + usize::from(state.write.probability > 0.0)
                + usize::from(state.alloc.probability > 0.0)
                + usize::from(state.send.probability > 0.0),
        };
        *state = GlobalState::new();
        cleared
    });

    ClearedFaults {
        mmap: global_cleared.mmap + local_cleared.mmap,
        read: global_cleared.read + local_cleared.read,
        write: global_cleared.write + local_cleared.write,
        alloc: global_cleared.alloc + local_cleared.alloc,
        send: global_cleared.send + local_cleared.send,
        persistent: global_cleared.persistent + local_cleared.persistent,
        probabilistic: global_cleared.probabilistic + local_cleared.probabilistic,
    }
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
#[must_use]
pub fn should_fail_mmap() -> bool {
    if thread_local_op_active(Operation::Mmap) {
        return with_thread_local_state_mut(|s| s.get_mut(Operation::Mmap).check());
    }
    if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
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
#[must_use]
pub fn should_fail_read() -> bool {
    if thread_local_op_active(Operation::Read) {
        return with_thread_local_state_mut(|s| s.get_mut(Operation::Read).check());
    }
    if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
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
#[must_use]
pub fn should_fail_write() -> bool {
    if thread_local_op_active(Operation::Write) {
        return with_thread_local_state_mut(|s| s.get_mut(Operation::Write).check());
    }
    if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
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
#[must_use]
pub fn should_fail_alloc() -> bool {
    if thread_local_op_active(Operation::Alloc) {
        return with_thread_local_state_mut(|s| s.get_mut(Operation::Alloc).check());
    }
    if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    // Reentrancy-safe path: the STATE lock is NOT reentrant, and an allocation
    // performed while this thread already holds it (e.g., a `Vec` growth inside
    // `try_inject_global`/`clear`) would re-enter here and self-deadlock on
    // `lock()`. `try_lock` returns `None` in that case (and under cross-thread
    // contention), so the nested/contended allocation is simply treated as
    // non-faulting rather than hanging the process.
    match STATE.try_lock() {
        Some(mut guard) => guard.get_mut(Operation::Alloc).check(),
        None => false,
    }
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
#[must_use]
pub fn should_fail_send() -> bool {
    if thread_local_op_active(Operation::Send) {
        return with_thread_local_state_mut(|s| s.get_mut(Operation::Send).check());
    }
    if !ENABLED.load(core::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    STATE.lock().get_mut(Operation::Send).check()
}
