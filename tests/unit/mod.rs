use crate::common::TEST_LOCK;

use faultkit::{
    clear, inject, inject_scoped, is_enabled, should_fail_alloc, should_fail_mmap,
    should_fail_read, should_fail_send, should_fail_write, try_inject, Fault, InjectionError,
    Operation,
};

#[test]
fn test_default_state() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert!(!is_enabled());
    assert!(!should_fail_mmap());
    assert!(!should_fail_read());
    assert!(!should_fail_write());
    assert!(!should_fail_alloc());
    assert!(!should_fail_send());
}

#[test]
fn test_clear_resets_state() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Mmap { fail_after: 0 }), Ok(()));
    assert!(is_enabled());

    let cleared = clear();
    assert_eq!(cleared.mmap, 1);
    assert_eq!(cleared.read, 0);

    assert!(!is_enabled());
    assert!(!should_fail_mmap());
}

#[test]
fn test_inject_duplicate_errors() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Mmap { fail_after: 5 }), Ok(()));
    let err = inject(Fault::Mmap { fail_after: 5 });
    assert_eq!(err, Err(InjectionError::DuplicateFailPoint));
}

#[test]
fn test_inject_scoped() {
    let _lock = TEST_LOCK.lock();
    clear();
    {
        let _guard = inject_scoped(Fault::Read { fail_after: 0 }).unwrap();
        assert!(is_enabled());
        assert!(should_fail_read());
    }
    assert!(!is_enabled());
    assert!(!should_fail_read());
}

#[test]
fn test_mmap_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Mmap { fail_after: 2 }), Ok(()));
    assert!(!should_fail_mmap()); // 0
    assert!(!should_fail_mmap()); // 1
    assert!(should_fail_mmap()); // 2
    assert!(!should_fail_mmap()); // 3
}

#[test]
fn test_read_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Read { fail_after: 1 }), Ok(()));
    assert!(!should_fail_read()); // 0
    assert!(should_fail_read()); // 1
    assert!(!should_fail_read()); // 2
}

#[test]
fn test_write_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Write { fail_after: 0 }), Ok(()));
    assert!(should_fail_write()); // 0
    assert!(!should_fail_write()); // 1
}

#[test]
fn test_alloc_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Alloc { fail_after: 3 }), Ok(()));
    assert!(!should_fail_alloc()); // 0
    assert!(!should_fail_alloc()); // 1
    assert!(!should_fail_alloc()); // 2
    assert!(should_fail_alloc()); // 3
    assert!(!should_fail_alloc()); // 4
}

#[test]
fn test_send_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(inject(Fault::Send { fail_after: 0 }), Ok(()));
    assert!(should_fail_send()); // 0
    assert!(!should_fail_send()); // 1
}

#[test]
fn test_probabilistic_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(
        try_inject(Fault::Probabilistic {
            op: Operation::Mmap,
            probability: 1.0,
        }),
        Ok(())
    );

    for _ in 0..100 {
        assert!(should_fail_mmap());
    }

    clear();
    assert_eq!(
        try_inject(Fault::Probabilistic {
            op: Operation::Mmap,
            probability: 0.0,
        }),
        Ok(())
    );

    for _ in 0..100 {
        assert!(!should_fail_mmap());
    }
}

#[test]
fn test_persistent_failure() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(
        try_inject(Fault::Persistent {
            op: Operation::Read,
            fail_after: 2,
        }),
        Ok(())
    );

    assert!(!should_fail_read()); // 0
    assert!(!should_fail_read()); // 1
    assert!(should_fail_read()); // 2
    assert!(should_fail_read()); // 3
    assert!(should_fail_read()); // 4
}

#[test]
fn test_multiple_failures() {
    let _lock = TEST_LOCK.lock();
    clear();
    assert_eq!(
        try_inject(Fault::Multiple {
            op: Operation::Write,
            fail_points: vec![1, 3],
        }),
        Ok(())
    );

    assert!(!should_fail_write()); // 0
    assert!(should_fail_write()); // 1
    assert!(!should_fail_write()); // 2
    assert!(should_fail_write()); // 3
    assert!(!should_fail_write()); // 4
}

#[test]
fn test_duplicate_fail_point_error() {
    let _lock = TEST_LOCK.lock();
    clear();
    let result = try_inject(Fault::Multiple {
        op: Operation::Alloc,
        fail_points: vec![5, 5],
    });
    assert_eq!(result, Err(InjectionError::DuplicateFailPoint));
}

#[test]
fn test_clear_returns_exact_counts() {
    let _lock = TEST_LOCK.lock();
    clear();

    assert_eq!(
        inject(Fault::Multiple {
            op: Operation::Mmap,
            fail_points: vec![1, 2, 3],
        }),
        Ok(())
    );

    assert_eq!(
        inject(Fault::Multiple {
            op: Operation::Read,
            fail_points: vec![5],
        }),
        Ok(())
    );

    let cleared = clear();
    assert_eq!(cleared.mmap, 3);
    assert_eq!(cleared.read, 1);
    assert_eq!(cleared.write, 0);
    assert_eq!(cleared.alloc, 0);
    assert_eq!(cleared.send, 0);
}

#[test]
fn test_multiple_operations_independently() {
    let _lock = TEST_LOCK.lock();
    clear();

    assert_eq!(inject(Fault::Mmap { fail_after: 0 }), Ok(()));
    assert_eq!(inject(Fault::Read { fail_after: 1 }), Ok(()));
    assert_eq!(inject(Fault::Write { fail_after: 2 }), Ok(()));

    // Mmap fails at 0
    assert!(should_fail_mmap());
    assert!(!should_fail_mmap());

    // Read fails at 1
    assert!(!should_fail_read());
    assert!(should_fail_read());
    assert!(!should_fail_read());

    // Write fails at 2
    assert!(!should_fail_write());
    assert!(!should_fail_write());
    assert!(should_fail_write());
    assert!(!should_fail_write());
}
#[test]
fn test_sequential_fault_injection_relative_offset() {
    let _lock = TEST_LOCK.lock();
    clear();

    // First injection: fail on 1st call (after 1 successful call)
    inject(Fault::Read { fail_after: 1 }).unwrap();
    assert!(!should_fail_read(), "call 0 should succeed");
    assert!(should_fail_read(), "call 1 should fail");
    // State now has calls = 2 and empty fail_points.

    // Second injection mid-execution: fail after 0 successful calls (i.e. next call)
    inject(Fault::Read { fail_after: 0 }).unwrap();
    assert!(
        should_fail_read(),
        "sequential inject(fail_after: 0) mid-execution must fail the very next call"
    );
    assert!(!should_fail_read(), "following call should succeed");
}

#[test]
fn test_persistent_fault_relative_offset_mid_execution() {
    let _lock = TEST_LOCK.lock();
    clear();

    // Execute 3 unfaulted calls by injecting fail_after: 2
    inject(Fault::Write { fail_after: 2 }).unwrap();
    assert!(!should_fail_write()); // call 0
    assert!(!should_fail_write()); // call 1
    assert!(should_fail_write()); // call 2 (faulted)
    // Write call count is now 3.

    // Inject persistent fault to start after 2 more successful calls from now
    inject(Fault::Persistent {
        op: Operation::Write,
        fail_after: 2,
    })
    .unwrap();

    // Calls 3 and 4 should succeed (2 calls post-injection)
    assert!(!should_fail_write(), "call 3 (1st post-inject) should succeed");
    assert!(!should_fail_write(), "call 4 (2nd post-inject) should succeed");

    // Call 5 and beyond must fail persistently
    assert!(should_fail_write(), "call 5 (3rd post-inject) must fail");
    assert!(should_fail_write(), "call 6 (4th post-inject) must fail");
}

#[test]
fn test_scoped_global_atomic_snapshot() {
    use faultkit::inject_scoped_global;
    let _lock = TEST_LOCK.lock();
    clear();

    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    assert!(is_enabled());

    {
        let _guard = inject_scoped_global(Fault::Read { fail_after: 0 }).unwrap();
        assert!(should_fail_mmap(), "outer Mmap fault still active");
        assert!(should_fail_read(), "inner scoped global Read fault active");
    }

    // Guard dropped: Read fault cleared, outer state restored
    assert!(!should_fail_read(), "Read fault removed after guard drop");
}
