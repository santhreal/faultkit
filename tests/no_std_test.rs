#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use faultkit::ClearedFaults;

#[test]
fn test_no_std_compiles() {
    let cleared = faultkit::clear();
    assert_eq!(cleared, ClearedFaults::default());
}
