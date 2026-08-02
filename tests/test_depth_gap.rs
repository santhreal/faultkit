// Serialize against every other test in this binary: faultkit's STATE is
// process-global, so tests that inject/clear must not interleave. Reuses the
// same static TEST_LOCK as the other suites.
#[path = "common/mod.rs"]
mod common;

use common::TEST_LOCK;
use faultkit::{
    clear, inject, inject_scoped, should_fail_alloc, should_fail_mmap, Fault, Operation,
};

#[test]
fn test_depth_gap_probability_one_reliably_fires_then_overwrite_disables() {
    let _g = TEST_LOCK.lock();
    clear();

    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.0,
    })
    .unwrap();
    // probability 1.0 fires deterministically: random_f64() is in [0.0, 1.0), so
    // `random < 1.0` is always true. (This was formerly #[ignore]'d as "does not
    // reliably trigger"; the check now makes it reliable.)
    assert!(should_fail_mmap());
    assert!(should_fail_mmap());

    // Re-injecting a probabilistic fault for the same op overwrites the previous
    // probability (documented behavior; try_inject does not stack probabilities).
    // Overwriting with 0.0 therefore disables the fault: probability 0.0 is not
    // active, so nothing fires and injection reports disabled.
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 0.0,
    })
    .unwrap();
    assert!(!should_fail_mmap());
}

#[test]
fn test_depth_gap_scoped_guard_preserves_outer_fault() {
    let _g = TEST_LOCK.lock();
    clear();

    // Outer fault on Alloc, injected OUTSIDE the scope.
    inject(Fault::Alloc { fail_after: 0 }).unwrap();

    {
        // Inner scoped fault on a DIFFERENT op (Mmap).
        let _guard = inject_scoped(Fault::Mmap { fail_after: 0 }).unwrap();
        assert!(should_fail_mmap(), "inner Mmap fault fires inside the scope");
        // Deliberately do NOT consume the Alloc fault here, so the assertion
        // after the scope actually measures whether the guard drop preserved it.
    }

    // The guard restores ONLY its own operation (Mmap) on drop, so the outer
    // Alloc fault is untouched and still fires. (Previously the guard called a
    // global clear() that wiped Alloc too - this asserts that bug stays fixed.)
    assert!(
        should_fail_alloc(),
        "the scoped Mmap guard must NOT clear the outer Alloc fault"
    );
    // And Mmap was restored to empty by the guard, so it no longer fires.
    assert!(!should_fail_mmap());
}
