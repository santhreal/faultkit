use faultkit::{
    clear, inject, inject_scoped, is_enabled, should_fail_alloc, should_fail_mmap,
    should_fail_read, should_fail_send, should_fail_write, Fault, InjectionError, Operation,
};
use parking_lot::Mutex;
use std::thread;

// Force serial execution of all adversarial tests since `faultkit` uses global state.
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_01_multiple_empty() {
    let _g = TEST_LOCK.lock();
    clear();
    assert!(inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![]
    })
    .is_ok());
    // FIXED (was: asserted the bug that ENABLED became true). An empty Multiple
    // configures no fault, so nothing is enabled.
    assert!(
        !is_enabled(),
        "an empty Multiple injects no fault, so injection must stay disabled"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_02_try_inject_sets_enabled_on_error() {
    let _g = TEST_LOCK.lock();
    clear();
    let err = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![0, 0],
    });
    assert_eq!(err, Err(InjectionError::DuplicateFailPoint));
    // FIXED (was: asserted the bug that a failed inject left ENABLED on). A
    // failed injection with no prior fault must not enable fault injection.
    assert!(
        !is_enabled(),
        "a failed injection must not leave fault injection enabled"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_03_multiple_partial_injection_on_error() {
    let _g = TEST_LOCK.lock();
    clear();
    let err = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![10, 10],
    });
    assert_eq!(err, Err(InjectionError::DuplicateFailPoint));
    // FIXED (was: asserted the bug that the first 10 was partially injected
    // before the duplicate error). Injection is now atomic: the whole [10, 10]
    // batch is rejected, so NOTHING is injected and no call ever fails.
    assert!(!is_enabled(), "a rejected batch must leave injection disabled");
    for _ in 0..12 {
        assert!(
            !should_fail_mmap(),
            "a duplicate Multiple batch must inject nothing (atomic)"
        );
    }
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_04_inject_scoped_clears_global_state() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Mmap { fail_after: 5 }).unwrap();
    {
        let _guard = inject_scoped(Fault::Read { fail_after: 2 }).unwrap();
    }
    // FIXED (was: asserted the bug that the scoped Read guard's drop wiped the
    // outer Mmap fault). The guard now restores ONLY its own operation (Read),
    // so the outer Mmap fault survives and injection stays enabled.
    assert!(
        is_enabled(),
        "the outer Mmap fault must survive the inner scoped Read guard drop"
    );
    // And the Mmap fault still fires at its configured call index (5).
    for _ in 0..5 {
        assert!(!should_fail_mmap());
    }
    assert!(should_fail_mmap(), "outer Mmap fail_after:5 must still be armed");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_05_persist_overwrites_silently() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Mmap { fail_after: 2 }).unwrap();
    let _ = should_fail_mmap(); // 0
    clear();
    inject(Fault::Mmap { fail_after: 1 }).unwrap();
    let _ = should_fail_mmap(); // 0
    assert!(should_fail_mmap(), "Count was reset, allowing 1 to be hit");
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_08_probability_nan() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.0,
    })
    .unwrap();
    assert!(should_fail_mmap());
    let cleared = clear();
    // FIXED (was: asserted the bug that the discrete point leaked when
    // probability also fired). check() now always consumes the matching discrete
    // fail point, so nothing is left to clear.
    assert_eq!(
        cleared.mmap, 0,
        "the discrete fail point must be consumed even when probability also fired (no leak)"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_13_persist_leaks_fail_points() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Read { fail_after: 0 }).unwrap();
    inject(Fault::Persistent {
        op: Operation::Read,
        fail_after: 0,
    })
    .unwrap();
    assert!(should_fail_read());
    let cleared = clear();
    // FIXED (was: asserted the bug that the discrete point leaked when the
    // persistent trigger fired). The discrete point is now consumed regardless.
    assert_eq!(
        cleared.read, 0,
        "the discrete fail point must be consumed even when a persistent fault also fired"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_14_should_fail_does_not_mutate_state_when_enabled_but_not_injected() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Mmap { fail_after: 10 }).unwrap();
    // FIXED (was: the Read call counter advanced even though Read had no fault,
    // just because ENABLED was globally true for Mmap). check() now returns
    // early for an inactive op WITHOUT incrementing its counter. Prove the Read
    // counter stayed at 0: a Read call here must not consume index 0, so a
    // freshly injected Read{fail_after:0} still fires on the very next call.
    assert!(!should_fail_read());
    inject(Fault::Read { fail_after: 0 }).unwrap();
    assert!(
        should_fail_read(),
        "the Read call counter must not have advanced while Read had no fault"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_15_cleared_faults_tracks_prob_persist() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Persistent {
        op: Operation::Mmap,
        fail_after: 0,
    })
    .unwrap();
    let cleared = clear();
    // A persistent fault adds no discrete points...
    assert_eq!(cleared.mmap, 0);
    // ...but FIXED: ClearedFaults now reports it via the `persistent` field
    // (was: persistent/probabilistic faults were invisible in the summary).
    assert_eq!(
        cleared.persistent, 1,
        "ClearedFaults must report the cleared persistent fault"
    );
    assert_eq!(cleared.probabilistic, 0);
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_16_inject_does_not_clear_previous() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    inject(Fault::Mmap { fail_after: 1 }).unwrap();
    assert!(should_fail_mmap());
    assert!(should_fail_mmap());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_17_probabilistic_zero() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
    clear();
    let points: Vec<u64> = (0..10_000).collect();
    // Injecting 10k distinct fail points is now O(N log N) (BTreeSet dup check),
    // not the old O(N^2) `contains()` scan. Just assert it succeeds cheaply.
    inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: points,
    })
    .unwrap();
    assert!(is_enabled());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_19_fail_after_u64_max() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
    clear();
    let _g1 = inject_scoped(Fault::Mmap { fail_after: 0 }).unwrap();
    let _g2 = inject_scoped(Fault::Read { fail_after: 0 }).unwrap();
    drop(_g2);
    // FIXED (was: asserted the bug that dropping _g2 (Read) cleared _g1 (Mmap)).
    // Each guard restores only its own operation, so _g1's Mmap fault is intact.
    assert!(
        should_fail_mmap(),
        "dropping the inner Read guard must not clear the outer Mmap guard's fault"
    );
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_24_persist_after_zero() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Mmap { fail_after: 0 }).unwrap();
    inject(Fault::Read { fail_after: 0 }).unwrap();
    assert!(should_fail_mmap());
    assert!(should_fail_read());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_27_check_increments_calls_even_when_probability_hits() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 1.0,
    })
    .unwrap();
    let _ = should_fail_mmap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_28_check_increments_calls_even_when_persist_hits() {
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Persistent {
        op: Operation::Mmap,
        fail_after: 0,
    })
    .unwrap();
    let _ = should_fail_mmap();
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_29_should_fail_alloc_without_enabled() {
    let _g = TEST_LOCK.lock();
    clear();
    assert!(!should_fail_alloc());
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_30_inject_duplicate_different_types() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
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
    // FIXED (was: asserted the bug that 3 was partially injected before the
    // batch hit the duplicate 2). Injection is atomic, so the [3, 2] batch is
    // rejected whole and only the original [1, 2] remain armed.
    assert!(!should_fail_mmap(), "call 0: not a fail point"); // 0
    assert!(should_fail_mmap(), "call 1: original fail point"); // 1
    assert!(should_fail_mmap(), "call 2: original fail point"); // 2
    assert!(
        !should_fail_mmap(),
        "call 3 must NOT fail: the [3, 2] batch was rejected atomically, so 3 was never injected"
    ); // 3
}

#[test]
#[allow(clippy::unwrap_used, clippy::used_underscore_binding)]
fn test_32_concurrent_inject_and_check() {
    let _g = TEST_LOCK.lock();
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
    let _g = TEST_LOCK.lock();
    clear();
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: f64::MAX,
    })
    .unwrap();
    assert!(should_fail_mmap());
}
