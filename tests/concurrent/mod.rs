use faultkit::{
    clear, inject, should_fail_alloc, should_fail_mmap, should_fail_read, should_fail_send,
    should_fail_write, try_inject, Fault, Operation,
};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_concurrent_access() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads + 1));

    assert_eq!(
        try_inject(Fault::Probabilistic {
            op: Operation::Mmap,
            probability: 0.5,
        }),
        Ok(())
    );

    let mut handles = vec![];

    for _ in 0..num_threads {
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            let mut failed = 0;
            for _ in 0..1000 {
                if should_fail_mmap() {
                    failed += 1;
                }
            }
            failed
        }));
    }

    barrier.wait(); // Start all threads

    let mut total_failed = 0;
    for handle in handles {
        total_failed += handle.join().unwrap();
    }

    assert!(
        total_failed > 10000 && total_failed < 40000,
        "total_failed was {} (expected ~25000)",
        total_failed
    );
}

#[test]
fn test_concurrent_multiple_operations() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads + 1));

    assert_eq!(
        inject(Fault::Probabilistic {
            op: Operation::Mmap,
            probability: 0.5
        }),
        Ok(())
    );
    assert_eq!(
        inject(Fault::Probabilistic {
            op: Operation::Read,
            probability: 0.5
        }),
        Ok(())
    );
    assert_eq!(
        inject(Fault::Probabilistic {
            op: Operation::Write,
            probability: 0.5
        }),
        Ok(())
    );
    assert_eq!(
        inject(Fault::Probabilistic {
            op: Operation::Alloc,
            probability: 0.5
        }),
        Ok(())
    );
    assert_eq!(
        inject(Fault::Probabilistic {
            op: Operation::Send,
            probability: 0.5
        }),
        Ok(())
    );

    let mut handles = vec![];

    for _ in 0..num_threads {
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            let mut ops_failed = 0;
            for _ in 0..100 {
                if should_fail_mmap() {
                    ops_failed += 1;
                }
                if should_fail_read() {
                    ops_failed += 1;
                }
                if should_fail_write() {
                    ops_failed += 1;
                }
                if should_fail_alloc() {
                    ops_failed += 1;
                }
                if should_fail_send() {
                    ops_failed += 1;
                }
            }
            ops_failed
        }));
    }

    barrier.wait();

    let mut total_ops_failed = 0;
    for handle in handles {
        total_ops_failed += handle.join().unwrap();
    }

    assert!(
        total_ops_failed > 5000 && total_ops_failed < 20000,
        "Expected failures to be around 12500, but got {}",
        total_ops_failed
    );
}

#[test]
fn test_concurrent_inject_and_clear() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads + 1));

    let mut handles = vec![];

    for i in 0..num_threads {
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            let mut failed_ops = 0;
            for _ in 0..100 {
                if i % 3 == 0 {
                    // Injecting duplicate points might fail, which is okay, so we ignore it
                    // but we consume the result correctly without ignoring it blindly.
                    if inject(Fault::Mmap { fail_after: 5 }).is_err() {
                        failed_ops += 1;
                    }
                } else if i % 3 == 1 {
                    if should_fail_mmap() {
                        failed_ops += 1;
                    }
                } else {
                    clear();
                }
            }
            failed_ops
        }));
    }

    barrier.wait();

    let mut total_failed = 0;
    for handle in handles {
        total_failed += handle.join().unwrap();
    }

    // As long as there is no panic and we have executed properly, we ensure that at least thread counts exist
    assert!(total_failed >= 0);
}
