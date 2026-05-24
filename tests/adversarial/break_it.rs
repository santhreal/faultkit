use faultkit::{
    clear, inject, inject_scoped, is_enabled, should_fail_alloc, should_fail_mmap,
    should_fail_read, should_fail_send, should_fail_write, Fault, InjectionError, Operation,
};
use std::sync::Mutex;
use std::thread;

// Force serial execution of all adversarial tests since `faultkit` uses global state.
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_01_multiple_empty() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    assert!(inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![]
    })
    .is_ok());
    assert!(
        is_enabled(),
        "Bug: ENABLED is true even without fail points"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_02_try_inject_sets_enabled_on_error() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    let err = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![0, 0],
    });
    assert_eq!(err, Err(InjectionError::DuplicateFailPoint));
    assert!(is_enabled(), "Bug: remained enabled despite error");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_03_multiple_partial_injection_on_error() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    let _ = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![10, 10],
    });
    for _ in 0..10 {
        assert!(!should_fail_mmap());
    }
    assert!(
        should_fail_mmap(),
        "Bug: The first 10 was partially injected before error"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_04_inject_scoped_clears_global_state() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 5 }).unwrap();
    {
        let _guard = inject_scoped(Fault::Read { fail_after: 2 }).unwrap();
    }
    assert!(
        !is_enabled(),
        "Bug: Global state was cleared by scoped guard"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_05_persist_overwrites_silently() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Persistent {
        op: Operation::Mmap,
        fail_after: 5,
    })
    .unwrap();
    inject(Fault::Persistent {
        op: Operation::Mmap,
        fail_after: 10,
    })
    .unwrap();
    // This succeeds instead of throwing DuplicateFailPoint or similar.
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_06_probability_overwrites_silently() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Read,
        probability: 0.5,
    })
    .unwrap();
    inject(Fault::Probabilistic {
        op: Operation::Read,
        probability: 0.9,
    })
    .unwrap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_07_clear_resets_call_counts() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 2 }).unwrap();
    should_fail_mmap(); // 0
    clear();
    inject(Fault::Mmap { fail_after: 1 }).unwrap();
    should_fail_mmap(); // 0
    assert!(should_fail_mmap(), "Count was reset, allowing 1 to be hit");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_08_probability_nan() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Write,
        probability: f64::NAN,
    })
    .unwrap();
    assert!(!should_fail_write(), "NaN should not trigger probability");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_09_probability_infinity() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Alloc,
        probability: f64::INFINITY,
    })
    .unwrap();
    assert!(should_fail_alloc(), "INFINITY should always trigger");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_10_probability_negative() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Send,
        probability: -1.0,
    })
    .unwrap();
    assert!(
        !should_fail_send(),
        "Negative probability should not trigger"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_11_probability_out_of_bounds() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.5,
    })
    .unwrap();
    assert!(should_fail_mmap(), "1.5 probability should trigger");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_12_probability_leaks_fail_points() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.0,
    })
    .unwrap();
    assert!(should_fail_mmap());
    let cleared = clear();
    assert_eq!(
        cleared.mmap, 1,
        "Bug: probability triggered true, bypassing point removal, leaking it"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_13_persist_leaks_fail_points() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Read { fail_after: 0 }).unwrap();
    inject(Fault::Persistent {
        op: Operation::Read,
        fail_after: 0,
    })
    .unwrap();
    assert!(should_fail_read());
    let cleared = clear();
    assert_eq!(
        cleared.read, 1,
        "Bug: persist triggered true, leaking fail point"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_14_should_fail_mutates_state_when_enabled_but_not_injected() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 10 }).unwrap();
    // Mutates internal call counter for Read, even though Read wasn't injected
    should_fail_read();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_15_cleared_faults_accuracy_missing_prob_persist() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Persistent {
        op: Operation::Mmap,
        fail_after: 0,
    })
    .unwrap();
    let cleared = clear();
    assert_eq!(
        cleared.mmap, 0,
        "Bug: ClearedFaults only tracks discrete fail points, missing persist"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_16_inject_does_not_clear_previous() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    inject(Fault::Mmap { fail_after: 1 }).unwrap();
    assert!(should_fail_mmap());
    assert!(should_fail_mmap());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_17_probabilistic_zero() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 0.0,
    })
    .unwrap();
    assert!(!should_fail_mmap(), "0.0 probability should not trigger");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_18_large_fail_points_resource_exhaustion_on_inject() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    let points: Vec<u64> = (0..10_000).collect();
    // Demonstrates O(N^2) complexity in inject due to contains loop
    inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: points,
    })
    .unwrap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_19_fail_after_u64_max() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap {
        fail_after: u64::MAX,
    })
    .unwrap();
    assert!(!should_fail_mmap());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_20_duplicate_persistent_fails() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Persistent {
        op: Operation::Read,
        fail_after: 5,
    })
    .unwrap();
    inject(Fault::Persistent {
        op: Operation::Write,
        fail_after: 5,
    })
    .unwrap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_21_multiple_fail_points_order() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Multiple {
        op: Operation::Alloc,
        fail_points: vec![1, 0],
    })
    .unwrap();
    assert!(should_fail_alloc());
    assert!(should_fail_alloc());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_22_concurrent_access_from_8_threads() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Persistent {
        op: Operation::Send,
        fail_after: 100,
    })
    .unwrap();
    let mut handles = vec![];
    for _ in 0..8 {
        handles.push(thread::spawn(|| {
            for _ in 0..1000 {
                let _ = should_fail_send();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_23_inject_scoped_multiple_times_clears_each_other() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    let _g1 = inject_scoped(Fault::Mmap { fail_after: 0 }).unwrap();
    let _g2 = inject_scoped(Fault::Read { fail_after: 0 }).unwrap();
    drop(_g2);
    assert!(!should_fail_mmap(), "Bug: _g2 drop cleared _g1");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_24_persist_after_zero() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Persistent {
        op: Operation::Alloc,
        fail_after: 0,
    })
    .unwrap();
    assert!(should_fail_alloc());
    assert!(should_fail_alloc());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_25_probability_one() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Write,
        probability: 1.0,
    })
    .unwrap();
    assert!(should_fail_write());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_26_multiple_points_same_value_different_operations() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    inject(Fault::Read { fail_after: 0 }).unwrap();
    assert!(should_fail_mmap());
    assert!(should_fail_read());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_27_check_increments_calls_even_when_probability_hits() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.0,
    })
    .unwrap();
    should_fail_mmap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_28_check_increments_calls_even_when_persist_hits() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Persistent {
        op: Operation::Mmap,
        fail_after: 0,
    })
    .unwrap();
    should_fail_mmap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_29_should_fail_alloc_without_enabled() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    assert!(!should_fail_alloc());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_30_inject_duplicate_different_types() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    let err = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![0],
    });
    assert!(err.is_err());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_31_multiple_partial_injection_across_calls() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![1, 2],
    })
    .unwrap();
    let err = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![3, 2],
    });
    assert!(err.is_err());
    // 3 gets partially injected before error on 2!
    should_fail_mmap(); // 0
    assert!(should_fail_mmap()); // 1
    assert!(should_fail_mmap()); // 2
    assert!(should_fail_mmap(), "Bug: 3 was partially injected!");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_32_concurrent_inject_and_check() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    let h1 = thread::spawn(|| {
        for i in 0..100 {
            let _ = inject(Fault::Mmap { fail_after: i });
        }
    });
    let h2 = thread::spawn(|| {
        for _ in 0..100 {
            let _ = should_fail_mmap();
        }
    });
    let _ = h1.join();
    let _ = h2.join();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_33_max_f64_probability() {
    let _g = TEST_LOCK.lock().unwrap();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: f64::MAX,
    })
    .unwrap();
    assert!(should_fail_mmap());
}
