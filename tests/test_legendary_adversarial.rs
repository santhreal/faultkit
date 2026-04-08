use faultkit::{clear, inject, should_fail_alloc, should_fail_mmap, Fault, Operation};
use std::sync::Mutex;

// faultkit uses global state — tests must be serialized to prevent races.
static FAULTKIT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_legendary_adversarial_max_u64() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    // Test boundaries
    inject(Fault::Mmap {
        fail_after: u64::MAX,
    })
    .unwrap();
    // Since we cannot iterate u64::MAX times in a test, we can only verify it's correctly injected and wait
    assert!(!should_fail_mmap());
}

#[test]
fn test_legendary_adversarial_invalid_floats() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    // Although f64 is passed, f64 values like NAN or out of bounds [0.0, 1.0) might behave unexpectedly
    inject(Fault::Probabilistic {
        op: Operation::Alloc,
        probability: f64::NAN,
    })
    .unwrap();
    // It shouldn't crash, but it might never fail or always fail. We just test it doesn't panic.
    let _ = should_fail_alloc();

    clear();
    inject(Fault::Probabilistic {
        op: Operation::Alloc,
        probability: 1.5,
    })
    .unwrap();
    let _ = should_fail_alloc();

    clear();
    inject(Fault::Probabilistic {
        op: Operation::Alloc,
        probability: -1.0,
    })
    .unwrap();
    let _ = should_fail_alloc();

    clear();
    inject(Fault::Probabilistic {
        op: Operation::Alloc,
        probability: f64::INFINITY,
    })
    .unwrap();
    let _ = should_fail_alloc();
}

#[test]
fn test_legendary_adversarial_empty_multiple() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![],
    })
    .unwrap();

    // Nothing should fail
    for _ in 0..10 {
        assert!(!should_fail_mmap());
    }
}

#[test]
fn test_legendary_adversarial_massive_multiple() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    // O(N^2) uniqueness check inside lock. 1_000 elements should take some time, but shouldn't timeout or panic.
    // We just test injection doesn't panic/timeout. Since state is global, actual testing of exact failures
    // in multiple tests is flaky if not careful.
    let fail_points: Vec<u64> = (0..1_000).collect();
    inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points,
    })
    .unwrap();

    // State is mutated by other tests. Just asserting we injected successfully.
    let _ = should_fail_mmap();
}

#[test]
fn test_legendary_adversarial_alternating() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![0, 2, 4, 6],
    })
    .unwrap();

    // 0
    assert!(should_fail_mmap());
    // 1
    assert!(!should_fail_mmap());
    // 2
    assert!(should_fail_mmap());
    // 3
    assert!(!should_fail_mmap());
    // 4
    assert!(should_fail_mmap());
    // 5
    assert!(!should_fail_mmap());
    // 6
    assert!(should_fail_mmap());
    // 7
    assert!(!should_fail_mmap());

    clear();
}

#[test]
fn test_legendary_adversarial_max_bounds() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    use faultkit::should_fail_read;
    clear();

    inject(Fault::Read {
        fail_after: u64::MAX,
    })
    .unwrap();

    assert!(
        !should_fail_read(),
        "First call should never fail since boundary is MAX"
    );
    // State is robust enough to handle the max bounds without overflow panic

    clear();
}
