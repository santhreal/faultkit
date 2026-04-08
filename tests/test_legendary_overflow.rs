use faultkit::{clear, inject, should_fail_alloc, should_fail_mmap, Fault, Operation};
use std::sync::Mutex;

static FAULTKIT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_overflow_u32_truncation() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    // Inject at u32::MAX + 1 to see if internal state truncates to 0
    let fail_point = (u32::MAX as u64) + 1;
    inject(Fault::Mmap {
        fail_after: fail_point,
    })
    .unwrap();

    // If it truncated to 0, this would fail immediately
    assert!(!should_fail_mmap(), "Should not truncate and fail early");

    // Ensure it doesn't fail early after a few loops
    for _ in 0..256 {
        assert!(
            !should_fail_mmap(),
            "Should not fail before hitting u32::MAX + 1"
        );
    }

    clear();
}

#[test]
fn test_exact_limits() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    // Exactly at common power-of-two boundaries
    inject(Fault::Multiple {
        op: Operation::Alloc,
        fail_points: vec![8, 16, 256],
    })
    .unwrap();

    for i in 0..=256 {
        let should_fail = should_fail_alloc();
        if i == 8 || i == 16 || i == 256 {
            assert!(should_fail, "Expected failure exactly at boundary {}", i);
        } else {
            assert!(!should_fail, "Expected success at {}, but it failed", i);
        }
    }

    clear();
}
