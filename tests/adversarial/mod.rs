use faultkit::{
    clear, should_fail_alloc, should_fail_mmap, try_inject, Fault, InjectionError, Operation,
};

#[test]
fn test_extreme_large_numbers() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    assert_eq!(
        try_inject(Fault::Mmap {
            fail_after: u64::MAX
        }),
        Ok(())
    );
    // It should not fail initially
    assert!(!should_fail_mmap());
}

#[test]
fn test_empty_multiple() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    assert_eq!(
        try_inject(Fault::Multiple {
            op: Operation::Mmap,
            fail_points: vec![],
        }),
        Ok(())
    );

    // Since there are no fail points, it should never fail
    for _ in 0..10 {
        assert!(!should_fail_mmap());
    }
}

#[test]
fn test_duplicate_points_in_multiple() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    let result = try_inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![1, 1, 2],
    });

    assert!(matches!(result, Err(InjectionError::DuplicateFailPoint)));
}

#[test]
fn test_invalid_probabilities() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    assert_eq!(
        try_inject(Fault::Probabilistic {
            op: Operation::Mmap,
            probability: -1.0,
        }),
        Ok(())
    );

    // Negative probability should result in 0% failures
    for _ in 0..100 {
        assert!(!should_fail_mmap());
    }

    clear();
    assert_eq!(
        try_inject(Fault::Probabilistic {
            op: Operation::Mmap,
            probability: 2.0, // Greater than 1.0
        }),
        Ok(())
    );

    // Probability > 1.0 should result in 100% failures
    for _ in 0..100 {
        assert!(should_fail_mmap());
    }
}

#[test]
fn test_integer_overflow_protection() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    // Since we can't easily iterate u64::MAX times, we test by injecting and seeing it doesn't panic
    assert_eq!(
        try_inject(Fault::Multiple {
            op: Operation::Alloc,
            fail_points: vec![u64::MAX],
        }),
        Ok(())
    );
    assert!(!should_fail_alloc());
}
