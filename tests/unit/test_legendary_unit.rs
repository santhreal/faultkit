use faultkit::{
    clear, inject, inject_scoped, is_enabled, should_fail_alloc, should_fail_mmap,
    should_fail_read, should_fail_send, should_fail_write, try_inject, ClearedFaults, Fault,
    InjectionError, Operation,
};
use std::sync::Mutex;

// faultkit uses process-global state — tests must be serialized.
static FAULTKIT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_legendary_unit_inject_and_clear() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    let _cleared = clear();

    assert!(!is_enabled());
    assert!(!should_fail_mmap());

    inject(Fault::Mmap { fail_after: 0 }).expect("failed to inject");

    assert!(is_enabled());
    assert!(should_fail_mmap());
    assert!(!should_fail_mmap()); // It only fails once!

    let cleared_after = clear();
    assert_eq!(cleared_after.mmap, 0); // Already consumed
}

#[test]
fn test_legendary_unit_try_inject_and_inject() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    try_inject(Fault::Read { fail_after: 2 }).expect("failed try_inject");
    inject(Fault::Read { fail_after: 3 }).expect("failed inject");

    assert!(!should_fail_read()); // 0
    assert!(!should_fail_read()); // 1
    assert!(should_fail_read()); // 2
    assert!(should_fail_read()); // 3
    assert!(!should_fail_read()); // 4
}

#[test]
fn test_legendary_unit_should_fail_all_types() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    inject(Fault::Mmap { fail_after: 1 }).unwrap();
    inject(Fault::Read { fail_after: 1 }).unwrap();
    inject(Fault::Write { fail_after: 1 }).unwrap();
    inject(Fault::Alloc { fail_after: 1 }).unwrap();
    inject(Fault::Send { fail_after: 1 }).unwrap();

    assert!(!should_fail_mmap());
    assert!(should_fail_mmap());

    assert!(!should_fail_read());
    assert!(should_fail_read());

    assert!(!should_fail_write());
    assert!(should_fail_write());

    assert!(!should_fail_alloc());
    assert!(should_fail_alloc());

    assert!(!should_fail_send());
    assert!(should_fail_send());

    let cleared = clear();
    assert_eq!(cleared, ClearedFaults::default());
}

#[test]
fn test_legendary_unit_duplicate_fail_point() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    let err1 = inject(Fault::Multiple {
        op: Operation::Mmap,
        fail_points: vec![5, 5],
    })
    .unwrap_err();
    assert_eq!(err1, InjectionError::DuplicateFailPoint);

    inject(Fault::Mmap { fail_after: 3 }).unwrap();
    let err2 = inject(Fault::Mmap { fail_after: 3 }).unwrap_err();
    assert_eq!(err2, InjectionError::DuplicateFailPoint);
}

#[test]
fn test_legendary_unit_inject_scoped() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    {
        let guard = inject_scoped(Fault::Read { fail_after: 0 }).unwrap();
        assert!(is_enabled());
        assert!(should_fail_read());
        drop(guard);
    }
    assert!(!is_enabled());
    assert!(!should_fail_read());
}

#[test]
fn test_legendary_unit_cleared_faults_struct() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();
    inject(Fault::Mmap { fail_after: 10 }).unwrap();
    inject(Fault::Mmap { fail_after: 11 }).unwrap();
    inject(Fault::Read { fail_after: 5 }).unwrap();

    let cleared = clear();
    assert!(cleared.mmap <= 2, "mmap cleared count should be at most 2");
    assert!(cleared.read <= 1, "read cleared count should be at most 1");
    assert_eq!(cleared.write, 0);
    assert_eq!(cleared.alloc, 0);
    assert_eq!(cleared.send, 0);
}
