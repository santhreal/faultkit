use faultkit::{
    clear, inject_scoped, should_fail_alloc, should_fail_read, should_fail_write, Fault,
};

// Mock structures to simulate a subsystem
struct StorageEngine;

impl StorageEngine {
    fn new() -> Self {
        if should_fail_alloc() {
            panic!("Mock OOM on init");
        }
        Self
    }

    fn write_record(&self) -> Result<(), &'static str> {
        if should_fail_write() {
            return Err("IO error during write");
        }
        Ok(())
    }

    fn read_record(&self) -> Result<(), &'static str> {
        if should_fail_read() {
            return Err("IO error during read");
        }
        Ok(())
    }
}

#[test]
fn test_storage_engine_resilience_loop() {
    let _lock = crate::common::TEST_LOCK.lock().unwrap();

    clear();

    // 1. Initial success
    let engine = StorageEngine::new();
    assert_eq!(engine.write_record(), Ok(()));

    // 2. Simulate scoped failure
    {
        let _fault_guard = inject_scoped(Fault::Write { fail_after: 0 }).unwrap();

        let result = engine.write_record();
        assert_eq!(result.unwrap_err(), "IO error during write");

        // Engine shouldn't fail reading while write is injected
        assert_eq!(engine.read_record(), Ok(()));
    }

    // 3. Post-guard, should succeed again
    assert_eq!(engine.write_record(), Ok(()));

    // 4. Simulate initialization failure
    {
        let _fault_guard = inject_scoped(Fault::Alloc { fail_after: 0 }).unwrap();
        let result = std::panic::catch_unwind(|| StorageEngine::new());
        assert!(
            result.is_err(),
            "Expected engine creation to fail due to alloc failure"
        );
    }

    // 5. Post-guard, should succeed initializing
    let new_engine = StorageEngine::new();
    assert_eq!(new_engine.write_record(), Ok(()));
}
