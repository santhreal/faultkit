use faultkit::{clear, inject, should_fail_alloc, Fault};
use std::sync::Mutex;

static FAULTKIT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_oom_injection_preserves_state() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    // Inject failure at the 3rd allocation call (fail_after: 2, meaning 0 and 1 succeed, 2 fails)
    inject(Fault::Alloc { fail_after: 2 }).unwrap();

    // Call 0 - should succeed
    assert!(!should_fail_alloc(), "0th call should not fail");

    // Call 1 - should succeed
    assert!(!should_fail_alloc(), "1st call should not fail");

    // Call 2 - should FAIL (OOM simulated)
    assert!(should_fail_alloc(), "2nd call MUST fail (OOM)");

    // Call 3+ - should succeed since the fail point was consumed
    assert!(
        !should_fail_alloc(),
        "3rd call should not fail, fail point consumed"
    );
    assert!(!should_fail_alloc(), "4th call should not fail");

    // Verify cleared correctly
    let cleared = clear();
    assert_eq!(cleared.alloc, 0, "Alloc failure should be consumed");
}

#[test]
fn test_oom_injection_multiple_preserves_state() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    inject(Fault::Alloc { fail_after: 0 }).unwrap();
    inject(Fault::Alloc { fail_after: 2 }).unwrap();

    assert!(should_fail_alloc(), "0th MUST fail");
    assert!(!should_fail_alloc(), "1st should succeed");
    assert!(should_fail_alloc(), "2nd MUST fail");
    assert!(!should_fail_alloc(), "3rd should succeed");

    clear();
}
