//! RAII guard for fault injection.

use crate::inject::{clear, try_inject};
use crate::types::{Fault, InjectionError};

/// RAII guard for fault injection.
#[derive(Debug)]
pub struct FaultGuard;

impl Drop for FaultGuard {
    fn drop(&mut self) {
        let _ = clear();
    }
}

/// Inject a fault and return an RAII guard that clears faults on drop.
///
/// # Errors
/// Returns an error if the fault injection fails.
pub fn inject_scoped(fault: Fault) -> Result<FaultGuard, InjectionError> {
    try_inject(fault)?;
    Ok(FaultGuard)
}
