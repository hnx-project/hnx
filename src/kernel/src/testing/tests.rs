// Prevents the testing module from being compiled in release builds
#![cfg(test)]

use crate::testing::*;

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

#[test_case]
fn another_trivial_assertion() {
    assert!(true);
}