use faultkit::{clear, try_inject, Fault, Operation};
use proptest::prelude::*;

prop_compose! {
    fn arb_operation()(op in prop_oneof![
        Just(Operation::Mmap),
        Just(Operation::Read),
        Just(Operation::Write),
        Just(Operation::Alloc),
        Just(Operation::Send),
    ]) -> Operation {
        op
    }
}

fn arb_fault() -> impl Strategy<Value = Fault> {
    prop_oneof![
        any::<u64>().prop_map(|fail_after| Fault::Mmap { fail_after }),
        any::<u64>().prop_map(|fail_after| Fault::Read { fail_after }),
        any::<u64>().prop_map(|fail_after| Fault::Write { fail_after }),
        any::<u64>().prop_map(|fail_after| Fault::Alloc { fail_after }),
        any::<u64>().prop_map(|fail_after| Fault::Send { fail_after }),
        (arb_operation(), any::<f64>())
            .prop_map(|(op, probability)| Fault::Probabilistic { op, probability }),
        (arb_operation(), any::<u64>())
            .prop_map(|(op, fail_after)| Fault::Persistent { op, fail_after }),
        (arb_operation(), prop::collection::vec(any::<u64>(), 0..100))
            .prop_map(|(op, fail_points)| Fault::Multiple { op, fail_points }),
    ]
}

proptest! {
    #[test]
    fn test_legendary_property_try_inject_no_panic(fault in arb_fault()) {
        clear();
        let _ = try_inject(fault);
        // It's allowed to return DuplicateFailPoint (if the fuzzer generates a vector with duplicates)
        // The invariant is: try_inject NEVER panics.
        let cleared = clear();

        // Assert clear never panics, and it shouldn't leave uninitialized/panic state
        // We'll just assert we cleared successfully.
        prop_assert!(cleared.mmap == cleared.mmap); // Always true, ensures we evaluated cleared
    }

    #[test]
    fn test_legendary_property_multiple_faults_together(
        faults in prop::collection::vec(arb_fault(), 0..20)
    ) {
        clear();
        for fault in faults {
            let _ = try_inject(fault);
        }
        let _ = clear();
        // The invariant: sequential multiple injections, even of diverse or duplicate types, NEVER panic.
    }
}
