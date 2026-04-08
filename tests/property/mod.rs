use crate::common::TEST_LOCK;

use faultkit::{clear, should_fail_mmap, try_inject, Fault, Operation};
use proptest::prelude::*;

proptest! {
    // Run tests with a single thread to avoid proptest interfering with itself due to global state
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_monotonicity_of_calls(fail_after in 100u64..500u64) {

        let _lock = TEST_LOCK.lock().unwrap();
        clear();
        assert_eq!(try_inject(Fault::Mmap { fail_after }), Ok(()));

        for _ in 0..fail_after {
            assert!(!should_fail_mmap());
        }
        assert!(should_fail_mmap());
        assert!(!should_fail_mmap());
    }

    #[test]
    fn test_persistent_stays_failed(fail_after in 50u64..200u64) {

        let _lock = TEST_LOCK.lock().unwrap();
        clear();
        assert_eq!(try_inject(Fault::Persistent {
            op: Operation::Mmap,
            fail_after,
        }), Ok(()));

        for _ in 0..fail_after {
            assert!(!should_fail_mmap());
        }

        // Must stay failed indefinitely
        for _ in 0..100 {
            assert!(should_fail_mmap());
        }
    }
}
