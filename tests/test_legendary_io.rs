use faultkit::{clear, inject, should_fail_read, should_fail_write, Fault};
use std::sync::Mutex;

static FAULTKIT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_io_read_injection() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    // Inject failure at the 2nd read call (fail_after: 1)
    inject(Fault::Read { fail_after: 1 }).unwrap();

    assert!(!should_fail_read(), "1st read call should not fail");
    assert!(should_fail_read(), "2nd read call MUST fail");
    assert!(!should_fail_read(), "3rd read call should not fail");

    let cleared = clear();
    assert_eq!(cleared.read, 0, "Read failure should be consumed");
}

#[test]
fn test_io_write_injection_partial() {
    let _lock = FAULTKIT_LOCK.lock().unwrap();
    clear();

    // Simulate a write loop chunking data
    // It takes 3 calls to write all data. We fail on the 3rd.
    inject(Fault::Write { fail_after: 2 }).unwrap();

    let chunks = vec![
        &b"chunk1"[..],
        &b"chunk2"[..],
        &b"chunk3"[..],
        &b"chunk4"[..],
    ];
    let mut written = 0;

    for (i, _chunk) in chunks.iter().enumerate() {
        if should_fail_write() {
            // Error!
            break;
        }
        written += 1;
        assert!(i < 2, "Should have broken before reaching the 3rd index");
    }

    assert_eq!(
        written, 2,
        "Should have successfully written exactly 2 chunks"
    );

    clear();
}
