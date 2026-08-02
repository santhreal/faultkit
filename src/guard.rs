//! RAII guard for fault injection.

use crate::config::{OpState, ENABLED, STATE, with_thread_local_state_mut};
use crate::inject::{try_inject, try_inject_global};
use crate::types::{Fault, InjectionError, Operation};

/// RAII guard for fault injection.
///
/// On drop it restores ONLY the operation it injected into, back to the state
/// that operation had before injection - it does not touch faults on other
/// operations or faults that other code injected concurrently.
#[derive(Debug)]
pub struct FaultGuard {
    op: Operation,
    snapshot: OpState,
    global: bool,
}

impl Drop for FaultGuard {
    fn drop(&mut self) {
        if self.global {
            // Restore ONLY this guard's operation to its pre-injection snapshot.
            let mut state = STATE.lock();
            *state.get_mut(self.op) = self.snapshot.clone();
            let any = state.any_active();
            drop(state);
            ENABLED.store(any, core::sync::atomic::Ordering::Relaxed);
        } else {
            // Thread-local scoped injection: restore the snapshot on the current
            // thread and leave the process-global state untouched.
            with_thread_local_state_mut(|state| {
                *state.get_mut(self.op) = self.snapshot.clone();
            });
        }
    }
}

/// Inject a thread-scoped fault and return an RAII guard that clears the fault on drop.
///
/// # Errors
/// Returns an error if the fault injection fails.
pub fn inject_scoped(fault: Fault) -> Result<FaultGuard, InjectionError> {
    let op = fault.operation();
    let snapshot = with_thread_local_state_mut(|state| state.get_mut(op).clone());
    try_inject(fault)?;
    Ok(FaultGuard {
        op,
        snapshot,
        global: false,
    })
}

/// Inject a global fault and return an RAII guard that clears the fault on drop.
///
/// This is the scoped equivalent of [`inject_global`]; the fault is visible to
/// all threads and the guard restores the previous global state for this
/// operation on drop.
///
/// # Errors
/// Returns an error if the fault injection fails.
pub fn inject_scoped_global(fault: Fault) -> Result<FaultGuard, InjectionError> {
    let op = fault.operation();
    let snapshot = STATE.lock().get_mut(op).clone();
    try_inject_global(fault)?;
    Ok(FaultGuard {
        op,
        snapshot,
        global: true,
    })
}
