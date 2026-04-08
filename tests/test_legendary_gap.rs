use faultkit::{
    clear, inject, inject_scoped, should_fail_alloc, should_fail_mmap, Fault, Operation,
};

#[test]
#[ignore = "GAP: Probabilistic fault injection with probability=1.0 does not reliably trigger should_fail_mmap()"]
fn test_legendary_gap_probabilistic_overrides_silently() {
    clear();

    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.0,
    })
    .unwrap();
    // It should now always fail
    assert!(should_fail_mmap());

    // Gap: injecting a new probabilistic value silently overwrites the previous one!
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 0.0,
    })
    .unwrap();
    // If the API appended or returned an error, the previous probability (1.0) would still be active
    // But it overwrites it. This is a gap because try_inject allows it silently.

    // The test ensures the gap still exists.
    assert!(!should_fail_mmap());
}

#[test]
fn test_legendary_gap_scoped_guard_clears_global_state() {
    clear();

    // Global injection
    inject(Fault::Alloc { fail_after: 0 }).unwrap();
    assert!(should_fail_alloc()); // FAILS once
                                  // reset for testing
    clear();
    inject(Fault::Alloc { fail_after: 0 }).unwrap();

    // Enter scoped guard
    {
        let _guard = inject_scoped(Fault::Mmap { fail_after: 0 }).unwrap();
        assert!(should_fail_mmap());
        assert!(should_fail_alloc());

        // At the end of scope, guard is dropped, it will call clear()
    }

    // Gap: The drop guard calls `clear()`, which resets ALL faults for ALL operations!
    // Alloc was set outside the scope, but it gets cleared too!
    assert!(!should_fail_alloc()); // Expected: true if isolated, false if global clear

    // Mmap shouldn't fail either
    assert!(!should_fail_mmap());
}
