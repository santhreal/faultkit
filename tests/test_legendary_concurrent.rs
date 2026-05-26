use std::sync::{Arc, Barrier, Mutex, OnceLock};
use std::thread;

use faultkit::{clear, inject, should_fail_mmap, Fault, Operation};

// Both tests in this file mutate faultkit's global fault state via
// `inject` + `clear` and observe it via `should_fail_mmap`. Default
// `cargo test` runs tests in parallel, which lets the two tests
// interleave their injects and observations — under load this drifts
// the probabilistic hammer's failure count outside its [2000, 4500]
// envelope (32000 calls × 0.1 expected, σ ≈ 54, but cross-test
// contamination can shift the count by hundreds). Serialize within
// this file with a static mutex.
fn serial_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_legendary_concurrent_hammer() {
    let _guard = serial_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear();

    // Inject a failure at call 500
    inject(Fault::Mmap { fail_after: 500 }).unwrap();

    let num_threads = 32;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = vec![];

    for _ in 0..num_threads {
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let mut failed = false;
            // 1000 iterations per thread * 32 threads = 32,000 calls total
            for _ in 0..1000 {
                if should_fail_mmap() {
                    failed = true;
                }
            }
            failed
        }));
    }

    let mut num_failed = 0;
    for handle in handles {
        if handle.join().unwrap() {
            num_failed += 1;
        }
    }

    // Exact count of failures should be 1 because Mmap fails only at the exact 500th call
    assert!(
        num_failed > 0,
        "Expected at least one thread to observe the failure"
    );

    let cleared = clear();
    assert_eq!(cleared.mmap, 0, "The failure should have been consumed");
}

#[test]
fn test_legendary_concurrent_probabilistic_hammer() {
    let _guard = serial_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear();

    // Inject a 10% probability of failure
    inject(Fault::Probabilistic {
        op: Operation::Mmap,
        probability: 0.1,
    })
    .unwrap();

    let num_threads = 32;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = vec![];

    for _ in 0..num_threads {
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let mut failed_count = 0;
            // 1000 iterations per thread
            for _ in 0..1000 {
                if should_fail_mmap() {
                    failed_count += 1;
                }
            }
            failed_count
        }));
    }

    let mut total_failed = 0;
    for handle in handles {
        total_failed += handle.join().unwrap();
    }

    // 32000 total calls with 0.1 probability should be around 3200 fails
    // We check that it's in a reasonable bound to account for randomness and no panics
    assert!(total_failed > 2000, "Too few failures: {}", total_failed);
    assert!(total_failed < 4500, "Too many failures: {}", total_failed);

    clear();
}
